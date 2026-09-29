use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use serde_json::{Value, json};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::storage::Database;

const START_DELAY: Duration = Duration::from_secs(5 * 60);
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(4 * 60);

#[derive(Clone)]
pub struct DatabaseDiagnosticsService {
    database: Database,
    state: Arc<RwLock<Value>>,
    started: Arc<AtomicBool>,
}

impl DatabaseDiagnosticsService {
    pub fn new(database: Database) -> Self {
        Self {
            database,
            state: Arc::new(RwLock::new(json!({
                "status": "WAITING",
                "scheduledAt": null,
                "startedAt": null,
                "completedAt": null,
                "report": null,
                "errorCode": null,
            }))),
            started: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        self.start_after(START_DELAY);
    }

    fn start_after(&self, delay: Duration) {
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let service = self.clone();
        tokio::spawn(async move {
            let scheduled_at = unix_timestamp().saturating_add(delay.as_secs() as i64);
            service.state.write().await["scheduledAt"] = json!(scheduled_at);
            tokio::time::sleep(delay).await;
            service.collect().await;
        });
    }

    pub async fn status(&self) -> Value {
        let mut status = self.state.read().await.clone();
        if let Some(object) = status.as_object_mut() {
            object.remove("report");
        }
        status
    }

    pub async fn report(&self) -> Option<Value> {
        let state = self.state.read().await;
        (state["status"] == "READY")
            .then(|| state["report"].clone())
            .filter(|report| !report.is_null())
    }

    async fn collect(&self) {
        let started_at = unix_timestamp();
        let scheduled_at = self.state.read().await["scheduledAt"]
            .as_i64()
            .unwrap_or(started_at);
        *self.state.write().await = json!({
            "status": "RUNNING",
            "scheduledAt": scheduled_at,
            "startedAt": started_at,
            "completedAt": null,
            "report": null,
            "errorCode": null,
        });
        let result = tokio::time::timeout(COLLECTION_TIMEOUT, async {
            let mut report = self
                .database
                .collect_database_diagnostics()
                .await
                .map_err(|_| ())?;
            let schema_version = self.database.schema_version().await.map_err(|_| ())?;
            report.details["reportVersion"] = json!(1);
            report.details["schemaVersion"] = json!(schema_version);
            report.details["generatedAt"] = json!(report.collected_at);
            Ok::<Value, ()>(report.details)
        })
        .await;
        let completed_at = unix_timestamp();
        match result {
            Ok(Ok(report)) => {
                *self.state.write().await = json!({
                    "status": "READY",
                    "scheduledAt": scheduled_at,
                    "startedAt": started_at,
                    "completedAt": completed_at,
                    "report": report,
                    "errorCode": null,
                });
                info!("read-only database diagnostics report is ready");
            }
            Ok(Err(())) => {
                self.mark_failed(scheduled_at, started_at, completed_at, "DIAGNOSTICS_FAILED")
                    .await
            }
            Err(_) => {
                self.mark_failed(
                    scheduled_at,
                    started_at,
                    completed_at,
                    "DIAGNOSTICS_TIMEOUT",
                )
                .await
            }
        }
    }

    async fn mark_failed(
        &self,
        scheduled_at: i64,
        started_at: i64,
        completed_at: i64,
        error_code: &str,
    ) {
        *self.state.write().await = json!({
            "status": "FAILED",
            "scheduledAt": scheduled_at,
            "startedAt": started_at,
            "completedAt": completed_at,
            "report": null,
            "errorCode": error_code,
        });
        warn!(error_code, "read-only database diagnostics report failed");
    }

    #[cfg(test)]
    pub(crate) fn start_immediately_for_test(&self) {
        self.start_after(Duration::ZERO);
    }
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn diagnostics_state_starts_waiting_and_report_is_absent() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let config = Config {
            http_addr: "127.0.0.1:8097".parse().expect("address"),
            config_dir: temp_dir.path().join("config"),
        };
        let database = Database::connect(&config).await.expect("database");
        let service = DatabaseDiagnosticsService::new(database.clone());
        let status = service.status().await;

        assert_eq!(status["status"], "WAITING");
        assert!(status["scheduledAt"].is_null());
        assert!(service.report().await.is_none());

        service.start_immediately_for_test();
        let status = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let status = service.status().await;
                if status["status"] == "READY" {
                    break status;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("report becomes ready");
        let report = service.report().await.expect("generated report");
        assert_eq!(report["backend"], "SQLITE");
        assert_eq!(status["status"], "READY");
        assert!(status["scheduledAt"].as_i64().is_some());
        assert!(status["completedAt"].as_i64().is_some());
        database.close().await;
    }
}
