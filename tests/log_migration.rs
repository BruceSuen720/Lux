use std::{env, error::Error};

use luxd::{
    application::{libraries::LibraryService, scanner::ScanJobService},
    config::{Config, DatabaseConfiguration, PostgresConnection},
    library::LibraryKind,
    observability::logs::{LogDateRange, LogExport, LogStore, export_logs},
    storage::Database,
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

async fn create_scan_job() -> Result<(tempfile::TempDir, Config, Database, String), Box<dyn Error>>
{
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let database = Database::connect(&config).await?;
    let job_id = create_scan_job_in(temp_dir.path(), &database).await?;
    Ok((temp_dir, config, database, job_id))
}

async fn create_scan_job_in(
    workspace: &std::path::Path,
    database: &Database,
) -> Result<String, Box<dyn Error>> {
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Migration", LibraryKind::Movie, false)
        .await?;
    let root = workspace.join("Movies");
    tokio::fs::create_dir_all(&root).await?;
    libraries
        .add_root(library.id, root.to_str().ok_or("non-UTF-8 root")?)
        .await?;
    let job = ScanJobService::new(database.clone())
        .create_movie_scan_job(library.id)
        .await?;
    Ok(job.id)
}

fn postgres_connection(database: String) -> PostgresConnection {
    PostgresConnection {
        host: env::var("POSTGRES_TEST_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
        port: env::var("POSTGRES_TEST_PORT")
            .unwrap_or_else(|_| "55432".to_owned())
            .parse()
            .unwrap_or(55432),
        database,
        username: env::var("POSTGRES_TEST_USER").unwrap_or_else(|_| "lux".to_owned()),
        password: env::var("POSTGRES_TEST_PASSWORD")
            .unwrap_or_else(|_| "lux-test-password".to_owned()),
        ssl_mode: "disable".to_owned(),
    }
}

async fn create_postgres_test_database() -> Result<(DatabaseConfiguration, String), Box<dyn Error>>
{
    let database_name = format!("lux_test_{}", Uuid::now_v7().simple());
    let admin_configuration =
        DatabaseConfiguration::Postgres(postgres_connection("postgres".to_owned()));
    let admin_url = admin_configuration
        .postgres_url()?
        .ok_or("missing PostgreSQL URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE DATABASE {database_name}"
    )))
    .execute(&pool)
    .await?;
    pool.close().await;
    Ok((
        DatabaseConfiguration::Postgres(postgres_connection(database_name.clone())),
        database_name,
    ))
}

async fn drop_postgres_test_database(database_name: &str) -> Result<(), Box<dyn Error>> {
    let admin_configuration =
        DatabaseConfiguration::Postgres(postgres_connection("postgres".to_owned()));
    let admin_url = admin_configuration
        .postgres_url()?
        .ok_or("missing PostgreSQL URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS {database_name}"
    )))
    .execute(&pool)
    .await?;
    pool.close().await;
    Ok(())
}

async fn run_postgres_scan_event_migration(
    config: &Config,
    database_configuration: &DatabaseConfiguration,
) -> Result<(), Box<dyn Error>> {
    let database = Database::connect_with_configuration(config, database_configuration).await?;
    let result: Result<(), Box<dyn Error>> = async {
        let job_id = create_scan_job_in(
            config.config_dir.parent().ok_or("missing config parent")?,
            &database,
        )
        .await?;
        sqlx::query(
            "INSERT INTO scan_job_events
                (id, job_id, level, event_code, message, details_json, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind("postgres-legacy-event")
        .bind(&job_id)
        .bind("WARN")
        .bind("SCAN_IO")
        .bind("legacy event")
        .bind(r#"{"attempt":2,"token":"must-not-leak"}"#)
        .bind(1_700_000_001_i64)
        .execute(database.pool())
        .await?;

        assert_eq!(database.migrate_legacy_scan_job_events_to_logs().await?, 1);
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_job_events")
            .fetch_one(database.pool())
            .await?;
        assert_eq!(remaining, 0);
        let (total, events) = LogStore::new(&config.config_dir)
            .list_scan_job_events(&job_id, None, None, 0, 10)
            .await?;
        assert_eq!(total, 1);
        assert_eq!(events[0].id, "postgres-legacy-event");
        assert_eq!(events[0].job_id, job_id);
        assert_eq!(events[0].level, "WARN");
        assert_eq!(events[0].event_code, "SCAN_IO");
        assert_eq!(events[0].message, "legacy event");
        assert_eq!(events[0].created_at, 1_700_000_001);
        let details: Value = serde_json::from_str(&events[0].details_json)?;
        assert_eq!(details["attempt"], 2);
        assert_eq!(details["token"], "[REDACTED]");
        Ok(())
    }
    .await;
    database.close().await;
    result
}

async fn insert_legacy_event(
    database: &Database,
    job_id: &str,
    id: &str,
    level: &str,
    code: &str,
    created_at: i64,
) -> Result<(), Box<dyn Error>> {
    sqlx::query(
        "INSERT INTO scan_job_events
            (id, job_id, level, event_code, message, details_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(job_id)
    .bind(level)
    .bind(code)
    .bind("legacy event")
    .bind(r#"{"attempt":2,"token":"must-not-leak"}"#)
    .bind(created_at)
    .execute(database.pool())
    .await?;
    Ok(())
}

#[tokio::test]
async fn scan_event_migration_without_database_history_is_a_noop() -> Result<(), Box<dyn Error>> {
    let (_temp_dir, _config, database, _job_id) = create_scan_job().await?;
    assert_eq!(database.migrate_legacy_scan_job_events_to_logs().await?, 0);
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_job_events")
        .fetch_one(database.pool())
        .await?;
    assert_eq!(remaining, 0);
    Ok(())
}

#[tokio::test]
#[ignore = "requires a local PostgreSQL instance"]
async fn postgres_legacy_scan_events_are_migrated_to_config_logs() -> Result<(), Box<dyn Error>> {
    let (database_configuration, database_name) = create_postgres_test_database().await?;
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let result = run_postgres_scan_event_migration(&config, &database_configuration).await;
    drop_postgres_test_database(&database_name).await?;
    result?;
    Ok(())
}

#[tokio::test]
async fn scan_event_migration_respects_archive_retention_across_date_batches()
-> Result<(), Box<dyn Error>> {
    let (_temp_dir, config, database, job_id) = create_scan_job().await?;
    let first_event_at = 1_700_000_000_i64;
    for day in 0..22 {
        insert_legacy_event(
            &database,
            &job_id,
            &format!("legacy-retention-{day:02}"),
            "INFO",
            "SCAN_PROGRESS",
            first_event_at + day * 86_400,
        )
        .await?;
    }

    assert_eq!(database.migrate_legacy_scan_job_events_to_logs().await?, 22);
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_job_events")
        .fetch_one(database.pool())
        .await?;
    assert_eq!(remaining, 0);

    let archive_dir = config.config_dir.join("logs").join("archive");
    let mut archives = tokio::fs::read_dir(archive_dir).await?;
    let mut archive_count = 0;
    while archives.next_entry().await?.is_some() {
        archive_count += 1;
    }
    assert_eq!(archive_count, 20);

    let (_, events) = LogStore::new(&config.config_dir)
        .list_scan_job_events(&job_id, None, None, 0, 100)
        .await?;
    let retained_legacy_events = events
        .iter()
        .filter(|event| event.id.starts_with("legacy-retention-"))
        .count();
    assert_eq!(retained_legacy_events, 21);
    Ok(())
}

#[tokio::test]
async fn legacy_scan_events_are_written_before_database_rows_are_deleted()
-> Result<(), Box<dyn Error>> {
    let (_temp_dir, config, database, job_id) = create_scan_job().await?;
    insert_legacy_event(
        &database,
        &job_id,
        "legacy-warn",
        "WARN",
        "SCAN_IO",
        1_700_000_001,
    )
    .await?;
    insert_legacy_event(
        &database,
        &job_id,
        "legacy-error",
        "ERROR",
        "PROBE_FAILED",
        1_700_000_002,
    )
    .await?;
    sqlx::query(
        "UPDATE scan_jobs SET cursor = ?, processed_count = 7, total_count = 9 WHERE id = ?",
    )
    .bind("resume-cursor")
    .bind(&job_id)
    .execute(database.pool())
    .await?;
    let before: (String, Option<String>, i64, i64) = sqlx::query_as(
        "SELECT status, cursor, processed_count, total_count FROM scan_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_one(database.pool())
    .await?;

    let migrated = database.migrate_legacy_scan_job_events_to_logs().await?;
    assert_eq!(migrated, 2);
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_job_events")
        .fetch_one(database.pool())
        .await?;
    assert_eq!(remaining, 0);
    let after: (String, Option<String>, i64, i64) = sqlx::query_as(
        "SELECT status, cursor, processed_count, total_count FROM scan_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_one(database.pool())
    .await?;
    assert_eq!(after, before);

    let (total, events) = LogStore::new(&config.config_dir)
        .list_scan_job_events(&job_id, None, None, 0, 10)
        .await?;
    assert_eq!(total, 2);
    assert_eq!(events[0].id, "legacy-error");
    assert_eq!(events[0].job_id, job_id);
    assert_eq!(events[0].level, "ERROR");
    assert_eq!(events[0].event_code, "PROBE_FAILED");
    assert_eq!(events[0].message, "legacy event");
    assert_eq!(events[0].created_at, 1_700_000_002);
    assert_eq!(events[1].id, "legacy-warn");
    assert_eq!(events[1].job_id, job_id);
    assert_eq!(events[1].level, "WARN");
    assert_eq!(events[1].event_code, "SCAN_IO");
    assert_eq!(events[1].message, "legacy event");
    let details: Value = serde_json::from_str(&events[0].details_json)?;
    assert_eq!(details["token"], "[REDACTED]");
    let legacy_date = time::OffsetDateTime::from_unix_timestamp(1_700_000_002)?.date();
    let LogExport::Daily { contents, .. } = export_logs(
        &config.config_dir,
        LogDateRange::new(legacy_date, legacy_date)?,
    )
    .await?
    else {
        return Err("single-day legacy export should remain JSONL".into());
    };
    let contents = String::from_utf8(contents)?;
    assert!(contents.contains("legacy-warn"));
    assert!(contents.contains("legacy-error"));
    Ok(())
}

#[tokio::test]
async fn scan_event_migration_preserves_rows_when_file_logging_fails_and_is_retryable()
-> Result<(), Box<dyn Error>> {
    let (_temp_dir, config, database, job_id) = create_scan_job().await?;
    insert_legacy_event(
        &database,
        &job_id,
        "legacy-retry",
        "ERROR",
        "SCAN_IO",
        1_700_000_003,
    )
    .await?;
    let blocked_log_directory = config.config_dir.join("logs");
    tokio::fs::write(&blocked_log_directory, b"not a directory").await?;

    assert!(
        database
            .migrate_legacy_scan_job_events_to_logs()
            .await
            .is_err()
    );
    let preserved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_job_events")
        .fetch_one(database.pool())
        .await?;
    assert_eq!(preserved, 1);

    tokio::fs::remove_file(&blocked_log_directory).await?;
    LogStore::open(&config.config_dir)
        .await?
        .append_scan_job_event(
            "legacy-retry",
            &job_id,
            "ERROR",
            "SCAN_IO",
            "legacy event",
            r#"{"attempt":2,"token":"must-not-leak"}"#,
        )
        .await?;
    assert_eq!(database.migrate_legacy_scan_job_events_to_logs().await?, 1);
    assert_eq!(database.migrate_legacy_scan_job_events_to_logs().await?, 0);
    let (total, events) = LogStore::new(&config.config_dir)
        .list_scan_job_events(&job_id, None, None, 0, 10)
        .await?;
    assert_eq!(total, 1);
    assert_eq!(events[0].id, "legacy-retry");
    Ok(())
}
