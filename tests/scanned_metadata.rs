use luxd::{
    application::{
        libraries::LibraryService,
        scanner::{IncrementalScanChange, ScanJobService},
        watch::ChangeKind,
    },
    config::Config,
    library::LibraryKind,
    storage::Database,
};

async fn wait_for_local_metadata_batches(
    database: &Database,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let (pending, failed): (i64, i64) = sqlx::query_as(
                "SELECT COUNT(*) FILTER (WHERE status IN ('PENDING', 'RUNNING')),
                        COUNT(*) FILTER (WHERE status = 'FAILED')
                 FROM scan_local_metadata_batches WHERE job_id = ?",
            )
            .bind(job_id)
            .fetch_one(database.pool())
            .await?;
            if failed > 0 {
                return Err(sqlx::Error::Protocol(format!(
                    "{failed} local metadata batch(es) failed"
                )));
            }
            if pending == 0 {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await??;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn manifest_scan_indexes_poster_while_local_nfo_is_blocked()
-> Result<(), Box<dyn std::error::Error>> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt, time::Duration};

    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Movies");
    let first_movie_dir = media_root.join("00 Slow NFO (2024)");
    tokio::fs::create_dir_all(&first_movie_dir).await?;
    tokio::fs::write(first_movie_dir.join("00.Slow.NFO.2024.mkv"), b"movie").await?;
    tokio::fs::write(first_movie_dir.join("poster.jpg"), b"poster").await?;
    let nfo_path = first_movie_dir.join("00.Slow.NFO.2024.nfo");
    let nfo_path_c = CString::new(nfo_path.as_os_str().as_bytes())?;
    // SAFETY: the path is a valid NUL-terminated filesystem path and mode is restrictive.
    let mkfifo_result = unsafe { libc::mkfifo(nfo_path_c.as_ptr(), 0o600) };
    assert_eq!(
        mkfifo_result, 0,
        "failed to create the blocking NFO fixture"
    );

    let second_movie_dir = media_root.join("01 Fast NFO (2024)");
    tokio::fs::create_dir_all(&second_movie_dir).await?;
    tokio::fs::write(second_movie_dir.join("01.Fast.NFO.2024.mkv"), b"movie").await?;
    tokio::fs::write(second_movie_dir.join("poster.jpg"), b"poster").await?;

    for index in 0..1_000 {
        let directory = media_root.join(format!("Later Movie {index:04} (2024)"));
        tokio::fs::create_dir_all(&directory).await?;
        tokio::fs::write(
            directory.join(format!("Later.Movie.{index:04}.2024.mkv")),
            b"movie",
        )
        .await?;
    }

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Movies", LibraryKind::Movie, false)
        .await?;
    libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?;

    let jobs = ScanJobService::new(database.clone());
    let job = jobs.create_movie_scan_job(library.id).await?;
    sqlx::query("CREATE TABLE test_local_metadata_scan_job (job_id TEXT NOT NULL)")
        .execute(database.pool())
        .await?;
    sqlx::query("CREATE TABLE test_local_metadata_claim_states (state TEXT NOT NULL)")
        .execute(database.pool())
        .await?;
    sqlx::query("INSERT INTO test_local_metadata_scan_job (job_id) VALUES (?)")
        .bind(&job.id)
        .execute(database.pool())
        .await?;
    sqlx::query(
        "CREATE TRIGGER capture_local_metadata_claim_state
         AFTER UPDATE OF status ON scan_local_metadata_batches
         WHEN NEW.status = 'RUNNING'
           AND NEW.job_id = (SELECT job_id FROM test_local_metadata_scan_job LIMIT 1)
         BEGIN
             INSERT INTO test_local_metadata_claim_states (state)
             SELECT state FROM scan_manifests WHERE job_id = NEW.job_id;
         END",
    )
    .execute(database.pool())
    .await?;
    let job_id = job.id.clone();
    let scan_jobs = jobs.clone();
    let mut scan = tokio::spawn(async move { scan_jobs.run_to_completion(&job_id, 1, None).await });

    let poster_visible_before_nfo = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let poster_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM item_images
                 WHERE item_images.local_path LIKE '%01 Fast NFO (2024)/poster.jpg'
                   AND item_images.image_type = 'POSTER'",
            )
            .fetch_one(database.pool())
            .await?;
            let local_batch_running: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM scan_local_metadata_batches
                 WHERE job_id = ? AND status = 'RUNNING'",
            )
            .bind(&job.id)
            .fetch_one(database.pool())
            .await?;
            if poster_count > 0 && local_batch_running > 0 {
                return Ok::<_, sqlx::Error>(true);
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or(Ok(false))?;

    let queue_state: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*) FROM scan_local_metadata_batches
         WHERE job_id = ? GROUP BY status ORDER BY status",
    )
    .bind(&job.id)
    .fetch_all(database.pool())
    .await?;
    let scan_state: (String, String) =
        sqlx::query_as("SELECT status, scan_phase FROM scan_jobs WHERE id = ?")
            .bind(&job.id)
            .fetch_one(database.pool())
            .await?;
    let image_paths: Vec<(String, String)> = sqlx::query_as(
        "SELECT item_images.image_type, item_images.local_path FROM item_images
         JOIN media_items ON media_items.id = item_images.item_id
         ORDER BY media_items.sort_title LIMIT 10",
    )
    .fetch_all(database.pool())
    .await?;
    let early_claim_states: Vec<String> =
        sqlx::query_scalar("SELECT state FROM test_local_metadata_claim_states")
            .fetch_all(database.pool())
            .await?;
    let scan_finished_before_nfo_unblocked =
        tokio::time::timeout(Duration::from_secs(30), &mut scan)
            .await
            .is_ok();

    // Release the NFO read whether the assertion passes or fails, so the worker can exit.
    tokio::fs::write(&nfo_path, b"<movie><title>Slow NFO</title></movie>").await?;
    if !scan_finished_before_nfo_unblocked {
        scan.await??;
    }
    wait_for_local_metadata_batches(&database, &job.id).await?;

    assert!(
        poster_visible_before_nfo,
        "a later poster should be indexed while the first NFO is blocked; scan={scan_state:?}, queue={queue_state:?}, images={image_paths:?}"
    );
    assert!(
        early_claim_states
            .iter()
            .any(|state| state == "DISCOVERING"),
        "the outbox worker should claim a batch during discovery; states={early_claim_states:?}"
    );
    assert!(
        scan_finished_before_nfo_unblocked,
        "scan completion should not wait for local NFO processing; scan={scan_state:?}, queue={queue_state:?}"
    );
    Ok(())
}

#[tokio::test]
async fn completed_movie_scan_indexes_local_nfo_and_images()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Movies");
    let movie_dir = media_root.join("Example Movie (2020)");
    tokio::fs::create_dir_all(&movie_dir).await?;
    tokio::fs::write(
        movie_dir.join("Example.Movie.2020.strm"),
        "https://example.invalid/media/example",
    )
    .await?;
    tokio::fs::write(
        movie_dir.join("Example.Movie.2020.nfo"),
        "<movie><title>Title From NFO</title><plot>Overview from NFO</plot><rating>8.4</rating></movie>",
    )
    .await?;
    tokio::fs::write(movie_dir.join("poster.jpg"), b"poster").await?;
    tokio::fs::write(movie_dir.join("fanart.jpg"), b"fanart").await?;

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Movies", LibraryKind::Movie, false)
        .await?;
    libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?;

    let jobs = ScanJobService::new(database.clone());
    let job = jobs.create_movie_scan_job(library.id).await?;
    jobs.run_to_completion(&job.id, 100, None).await?;
    wait_for_local_metadata_batches(&database, &job.id).await?;

    let item: (String, String, Option<f64>, Option<String>) = sqlx::query_as(
        "SELECT title, overview, rating, rating_source
         FROM media_items WHERE item_type = 'MOVIE'",
    )
    .fetch_one(database.pool())
    .await?;
    assert_eq!(
        item,
        (
            "Title From NFO".to_owned(),
            "Overview from NFO".to_owned(),
            Some(8.4),
            Some("NFO".to_owned()),
        )
    );

    let images: Vec<(String, String)> =
        sqlx::query_as("SELECT image_type, local_path FROM item_images ORDER BY image_type")
            .fetch_all(database.pool())
            .await?;
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].0, "FANART");
    assert_eq!(images[1].0, "POSTER");
    assert!(images.iter().all(|(_, path)| path.ends_with(".jpg")));
    Ok(())
}

#[tokio::test]
async fn incremental_movie_scan_indexes_local_images() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Movies");
    tokio::fs::create_dir_all(&media_root).await?;

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Movies", LibraryKind::Movie, false)
        .await?;
    let root = libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?
        .root;

    let movie_dir = media_root.join("Incremental Movie (2024)");
    tokio::fs::create_dir_all(&movie_dir).await?;
    tokio::fs::write(movie_dir.join("Incremental.Movie.2024.mkv"), b"movie").await?;
    tokio::fs::write(movie_dir.join("poster.jpg"), b"poster").await?;
    tokio::fs::write(movie_dir.join("fanart.jpg"), b"fanart").await?;

    let jobs = ScanJobService::new(database.clone());
    let job = jobs
        .enqueue_incremental_changes(
            library.id,
            vec![luxd::application::scanner::IncrementalScanChange {
                root_id: root.id.to_string(),
                relative_path: "Incremental Movie (2024)".to_owned(),
                kind: luxd::application::watch::ChangeKind::Create,
            }],
        )
        .await?;
    jobs.run_to_completion(&job.id, 100, None).await?;
    wait_for_local_metadata_batches(&database, &job.id).await?;

    let images: Vec<(String, String)> =
        sqlx::query_as("SELECT image_type, local_path FROM item_images ORDER BY image_type")
            .fetch_all(database.pool())
            .await?;
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].0, "FANART");
    assert_eq!(images[1].0, "POSTER");
    assert!(images.iter().all(|(_, path)| path.ends_with(".jpg")));
    Ok(())
}

#[tokio::test]
async fn incremental_sidecar_change_replaces_local_image() -> Result<(), Box<dyn std::error::Error>>
{
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Movies");
    let movie_dir = media_root.join("Sidecar Movie (2024)");
    tokio::fs::create_dir_all(&movie_dir).await?;
    tokio::fs::write(movie_dir.join("Sidecar.Movie.2024.mkv"), b"movie").await?;
    tokio::fs::write(movie_dir.join("poster.jpg"), b"old-poster").await?;

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Movies", LibraryKind::Movie, false)
        .await?;
    let root = libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?
        .root;
    let jobs = ScanJobService::new(database.clone());
    let initial = jobs.create_movie_scan_job(library.id).await?;
    jobs.run_to_completion(&initial.id, 100, None).await?;
    wait_for_local_metadata_batches(&database, &initial.id).await?;

    tokio::fs::remove_file(movie_dir.join("poster.jpg")).await?;
    tokio::fs::write(movie_dir.join("poster.webp"), b"new-poster").await?;
    let incremental = jobs
        .enqueue_incremental_changes(
            library.id,
            vec![IncrementalScanChange {
                root_id: root.id.to_string(),
                relative_path: "Sidecar Movie (2024)/poster.webp".to_owned(),
                kind: ChangeKind::Modify,
            }],
        )
        .await?;
    jobs.run_to_completion(&incremental.id, 100, None).await?;

    let poster_path: String =
        sqlx::query_scalar("SELECT local_path FROM item_images WHERE image_type = 'POSTER'")
            .fetch_one(database.pool())
            .await?;
    assert!(poster_path.ends_with("poster.webp"));
    Ok(())
}

#[tokio::test]
async fn movie_scan_indexes_multiple_emby_backdrops_in_order()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Movies");
    let movie_dir = media_root.join("Multiple Backdrops (2024)");
    tokio::fs::create_dir_all(&movie_dir).await?;
    tokio::fs::write(movie_dir.join("Multiple.Backdrops.2024.mkv"), b"movie").await?;
    tokio::fs::write(movie_dir.join("backdrop.jpg"), b"backdrop-0").await?;
    tokio::fs::write(movie_dir.join("backdrop1.jpg"), b"backdrop-1").await?;
    tokio::fs::write(movie_dir.join("fanart-2.jpg"), b"backdrop-2").await?;

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Movies", LibraryKind::Movie, false)
        .await?;
    libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?;

    let jobs = ScanJobService::new(database.clone());
    let job = jobs.create_movie_scan_job(library.id).await?;
    jobs.run_to_completion(&job.id, 100, None).await?;
    wait_for_local_metadata_batches(&database, &job.id).await?;

    let images: Vec<(i64, String)> = sqlx::query_as(
        "SELECT image_index, local_path
         FROM item_images
         WHERE image_type = 'FANART'
         ORDER BY image_index",
    )
    .fetch_all(database.pool())
    .await?;
    assert_eq!(images.len(), 3);
    assert_eq!(images[0].0, 0);
    assert!(images[0].1.ends_with("backdrop.jpg"));
    assert_eq!(images[1].0, 1);
    assert!(images[1].1.ends_with("backdrop1.jpg"));
    assert_eq!(images[2].0, 2);
    assert!(images[2].1.ends_with("fanart-2.jpg"));
    Ok(())
}

#[tokio::test]
async fn completed_flat_movie_scan_indexes_media_prefixed_images_per_item()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Movies");
    tokio::fs::create_dir_all(&media_root).await?;
    for (stem, poster, backdrop) in [
        ("Flat.One.2020", "poster.png", "fanart.jpg"),
        ("Flat.Two.2021", "poster.png", "backdrop.jpg"),
    ] {
        tokio::fs::write(
            media_root.join(format!("{stem}.strm")),
            "https://example.invalid/media/flat",
        )
        .await?;
        tokio::fs::write(media_root.join(format!("{stem}-{poster}")), b"poster").await?;
        tokio::fs::write(media_root.join(format!("{stem}-{backdrop}")), b"backdrop").await?;
    }

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Movies", LibraryKind::Movie, false)
        .await?;
    libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?;

    let jobs = ScanJobService::new(database.clone());
    let job = jobs.create_movie_scan_job(library.id).await?;
    jobs.run_to_completion(&job.id, 100, None).await?;
    wait_for_local_metadata_batches(&database, &job.id).await?;

    let images: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT media_items.sort_title, item_images.image_type, item_images.local_path
         FROM item_images
         JOIN media_items ON media_items.id = item_images.item_id
         ORDER BY media_items.sort_title, item_images.image_type",
    )
    .fetch_all(database.pool())
    .await?;
    assert_eq!(images.len(), 4);
    assert!(images.iter().any(|(title, image_type, path)| {
        title == "flat one" && image_type == "POSTER" && path.ends_with("Flat.One.2020-poster.png")
    }));
    assert!(images.iter().any(|(title, image_type, path)| {
        title == "flat one" && image_type == "FANART" && path.ends_with("Flat.One.2020-fanart.jpg")
    }));
    assert!(images.iter().any(|(title, image_type, path)| {
        title == "flat two" && image_type == "POSTER" && path.ends_with("Flat.Two.2021-poster.png")
    }));
    assert!(images.iter().any(|(title, image_type, path)| {
        title == "flat two"
            && image_type == "FANART"
            && path.ends_with("Flat.Two.2021-backdrop.jpg")
    }));
    Ok(())
}

#[tokio::test]
async fn rescan_updates_an_existing_local_image_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Movies");
    let movie_dir = media_root.join("Updated Movie (2026)");
    tokio::fs::create_dir_all(&movie_dir).await?;
    tokio::fs::write(movie_dir.join("Updated.Movie.2026.mkv"), b"movie").await?;
    tokio::fs::write(movie_dir.join("poster.jpg"), b"old-poster").await?;

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Movies", LibraryKind::Movie, false)
        .await?;
    libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?;

    let jobs = ScanJobService::new(database.clone());
    let first_job = jobs.create_movie_scan_job(library.id).await?;
    jobs.run_to_completion(&first_job.id, 100, None).await?;
    wait_for_local_metadata_batches(&database, &first_job.id).await?;
    let first_image: (String, Option<String>) = sqlx::query_as(
        "SELECT local_path, content_tag FROM item_images WHERE image_type = 'POSTER'",
    )
    .fetch_one(database.pool())
    .await?;

    tokio::fs::remove_file(movie_dir.join("poster.jpg")).await?;
    tokio::fs::write(movie_dir.join("poster.webp"), b"new-poster").await?;
    let second_job = jobs.create_movie_scan_job(library.id).await?;
    jobs.run_to_completion(&second_job.id, 100, None).await?;
    wait_for_local_metadata_batches(&database, &second_job.id).await?;

    let second_image: (String, Option<String>) = sqlx::query_as(
        "SELECT local_path, content_tag FROM item_images WHERE image_type = 'POSTER'",
    )
    .fetch_one(database.pool())
    .await?;
    assert!(second_image.0.ends_with("poster.webp"));
    assert!(first_image.1.is_some());
    assert!(second_image.1.is_some());
    assert_ne!(first_image.1, second_image.1);
    Ok(())
}

#[tokio::test]
async fn completed_mixed_scan_indexes_local_movie_and_series_images()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Mixed");
    let movie_dir = media_root.join("Example Movie (2020)");
    tokio::fs::create_dir_all(&movie_dir).await?;
    tokio::fs::write(
        movie_dir.join("Example.Movie.2020.strm"),
        "https://example.invalid/media/movie",
    )
    .await?;
    tokio::fs::write(movie_dir.join("poster.jpg"), b"movie-poster").await?;
    tokio::fs::write(movie_dir.join("fanart.jpg"), b"movie-fanart").await?;

    let series_dir = media_root.join("Example Show");
    let season_dir = series_dir.join("Season 01");
    tokio::fs::create_dir_all(&season_dir).await?;
    tokio::fs::write(
        series_dir.join("tvshow.nfo"),
        "<tvshow><title>Example Show</title></tvshow>",
    )
    .await?;
    tokio::fs::write(
        season_dir.join("Example.Show.S01E01.strm"),
        "https://example.invalid/media/episode",
    )
    .await?;
    tokio::fs::write(series_dir.join("poster.jpg"), b"series-poster").await?;
    tokio::fs::write(series_dir.join("fanart.jpg"), b"series-fanart").await?;

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Mixed", LibraryKind::Mixed, false)
        .await?;
    libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?;

    let jobs = ScanJobService::new(database.clone());
    let job = jobs.create_movie_scan_job(library.id).await?;
    jobs.run_to_completion(&job.id, 100, None).await?;
    wait_for_local_metadata_batches(&database, &job.id).await?;

    let images: Vec<(String, String)> = sqlx::query_as(
        "SELECT media_items.item_type, item_images.image_type
         FROM item_images
         JOIN media_items ON media_items.id = item_images.item_id
         ORDER BY media_items.item_type, item_images.image_type",
    )
    .fetch_all(database.pool())
    .await?;
    assert_eq!(
        images,
        vec![
            ("MOVIE".to_owned(), "FANART".to_owned()),
            ("MOVIE".to_owned(), "POSTER".to_owned()),
            ("SERIES".to_owned(), "FANART".to_owned()),
            ("SERIES".to_owned(), "POSTER".to_owned()),
        ]
    );
    Ok(())
}

#[tokio::test]
async fn failed_local_poster_insert_does_not_mark_image_stage_complete()
-> Result<(), Box<dyn std::error::Error>> {
    use std::io::Cursor;

    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let media_root = temp_dir.path().join("Movies");
    let movie_dir = media_root.join("Broken Poster Insert (2026)");
    tokio::fs::create_dir_all(&movie_dir).await?;
    tokio::fs::write(movie_dir.join("Broken.Poster.Insert.2026.mkv"), b"movie").await?;
    let mut poster_png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        1,
        1,
        image::Rgba([255, 0, 0, 255]),
    ))
    .write_to(&mut poster_png, image::ImageFormat::Png)?;
    tokio::fs::write(movie_dir.join("poster.png"), poster_png.into_inner()).await?;

    let database = Database::connect(&config).await?;
    let libraries = LibraryService::new(database.clone());
    let library = libraries
        .create_library("Movies", LibraryKind::Movie, false)
        .await?;
    libraries
        .add_root(library.id, media_root.to_str().ok_or("non-utf8 path")?)
        .await?;
    sqlx::query(
        "CREATE TRIGGER fail_local_poster_insert
         BEFORE INSERT ON item_images
         WHEN NEW.image_type = 'POSTER'
         BEGIN SELECT RAISE(ABORT, 'injected poster insert failure'); END",
    )
    .execute(database.pool())
    .await?;

    let jobs = ScanJobService::new(database.clone());
    jobs.start_local_metadata_outbox_worker().await?;
    let job = jobs.create_movie_scan_job(library.id).await?;
    jobs.run_to_completion(&job.id, 100, None).await?;

    let batch = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let batch: Option<(String, Option<i64>)> = sqlx::query_as(
                "SELECT status, images_completed_at FROM scan_local_metadata_batches
                 WHERE job_id = ? ORDER BY created_at, id LIMIT 1",
            )
            .bind(&job.id)
            .fetch_optional(database.pool())
            .await?;
            if batch
                .as_ref()
                .is_some_and(|(status, _)| status == "FAILED" || status == "COMPLETED")
            {
                return Ok::<_, sqlx::Error>(batch);
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await??;
    let (status, images_completed_at) = batch.ok_or("scan produced no local metadata batch")?;

    assert_eq!(status, "FAILED");
    assert_eq!(images_completed_at, None);
    Ok(())
}
