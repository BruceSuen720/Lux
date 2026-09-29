pub mod logs;
pub mod resources;

use std::path::Path;

use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub async fn init(config_dir: &Path) -> (Option<WorkerGuard>, Option<logs::LogStore>) {
    let log_store = match logs::LogStore::open(config_dir).await {
        Ok(log_store) => log_store,
        Err(error) => {
            eprintln!("Lux file logging unavailable; continuing with stdout logging: {error}");
            init_stdout();
            return (None, None);
        }
    };

    let (file_writer, guard) = NonBlockingBuilder::default()
        .thread_name("lux-log-writer")
        .finish(log_store.writer());
    let filter = env_filter();
    let stdout_layer = fmt::layer().json().with_writer(std::io::stdout);
    let file_layer = fmt::layer().json().with_writer(file_writer);
    if tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .try_init()
        .is_err()
    {
        drop(guard);
        return (None, Some(log_store));
    }
    (Some(guard), Some(log_store))
}

fn init_stdout() {
    let _ = fmt().json().with_env_filter(env_filter()).try_init();
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("luxd=info,tower_http=info"))
}
