use luxd::{
    api::{AppState, app_with_state},
    application::setup::SetupService,
    auth::{
        admin_api_key::AdminApiKeyService, emby::EmbyAuthService, sessions::WebAuthService,
        users::UserStore,
    },
    config::Config,
    storage::Database,
};
use tokio::net::TcpListener;

#[tokio::test]
async fn database_diagnostics_endpoints_require_admin_and_wait_until_report_is_ready()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = Config {
        http_addr: "127.0.0.1:8097".parse()?,
        config_dir: temp_dir.path().join("config"),
    };
    let database = Database::connect(&config).await?;
    UserStore::new(database.clone())?
        .create_initial_admin("Admin", "Administrator", "correct horse battery staple")
        .await?;
    let key = AdminApiKeyService::new(config.config_dir.clone(), database.clone())
        .rotate()
        .await?;
    let app = app_with_state(AppState::ready(
        config,
        database.clone(),
        SetupService::new(database.clone())?,
        WebAuthService::new(database.clone())?,
        EmbyAuthService::new(database.clone())?,
    ));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let client = reqwest::Client::new();

    let status_url = format!("http://{address}/api/v1/admin/database-diagnostics");
    let export_url = format!("http://{address}/api/v1/admin/database-diagnostics/export");
    assert_eq!(
        client.get(&status_url).send().await?.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let status = client
        .get(&status_url)
        .header("X-Lux-Api-Key", &key)
        .send()
        .await?;
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    assert_eq!(
        status.headers()[reqwest::header::CACHE_CONTROL],
        "no-store, max-age=0"
    );
    let status_body = status.text().await?;
    assert!(status_body.contains("WAITING"));
    assert!(!status_body.contains("report"));
    assert!(!status_body.contains(&key));

    let export = client
        .get(&export_url)
        .header("X-Lux-Api-Key", &key)
        .send()
        .await?;
    assert_eq!(export.status(), reqwest::StatusCode::CONFLICT);
    assert!(
        export
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .is_none()
    );
    let export_error = export.json::<serde_json::Value>().await?;
    assert_eq!(export_error["error"]["code"], "INVALID_REQUEST");

    server.abort();
    database.close().await;
    Ok(())
}
