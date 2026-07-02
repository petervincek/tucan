use std::{fs, sync::Arc};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::core::config::{ConfigManager, LoggingConfig};

pub struct LogManager {
    config_manager: Arc<ConfigManager>,
}

/// `LogManager` is responsible for initializing the application logger
impl LogManager {
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self { config_manager }
    }

    /// Sets up file logging and returns a WorkerGuard.
    /// Crucial: The returned WorkerGuard MUST live in main.rs to keep the file thread open!
    pub fn init(&self, logging_config: &LoggingConfig) -> anyhow::Result<WorkerGuard> {
        let log_dir = self.config_manager.get_config_dir();
        fs::create_dir_all(&log_dir)?;

        // Direct logging to write out to rotating file handles safely
        let file_appender =
            tracing_appender::rolling::daily(&log_dir, &logging_config.log_file_name);
        let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);

        // build the log filter with the required log level
        let filter_log_level = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(format!("tucan={}", logging_config.log_level)));

        let init_result = tracing_subscriber::registry()
            .with(filter_log_level)
            .with(
                fmt::layer()
                    .with_writer(non_blocking_writer)
                    .with_ansi(false),
            )
            .try_init();

        match init_result {
            Ok(_) => {}
            Err(err) => {
                let error_message = err.to_string();
                if error_message.contains("a global default trace dispatcher has already been set")
                {
                    // A global default subscriber is already set; continue with the existing one.
                } else {
                    return Err(err.into());
                }
            }
        }

        tracing::info!("Diagnostics logging framework ['tracing'] initialized successfully.");
        Ok(guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn init_creates_log_directory_and_returns_worker_guard() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let config_dir = temp_dir.path().join("tucan");
        let config_manager = Arc::new(ConfigManager::new(Some(config_dir.clone())));
        let log_manager = LogManager::new(config_manager.clone());

        let logging_config = LoggingConfig {
            log_file_name: String::from("test.log"),
            log_level: String::from("info"),
        };

        let guard = log_manager.init(&logging_config).expect("init failed");

        assert!(config_dir.exists(), "log directory should be created");
        let log_files: Vec<_> = std::fs::read_dir(&config_dir)
            .expect("read log directory failed")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("test.log"))
            .collect();
        assert!(
            !log_files.is_empty(),
            "expected at least one rolling log file"
        );

        drop(guard);
    }

    #[test]
    fn init_can_be_called_twice_without_failing() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let config_dir = temp_dir.path().join("tucan");
        let config_manager = Arc::new(ConfigManager::new(Some(config_dir.clone())));
        let log_manager = LogManager::new(config_manager.clone());

        let logging_config = LoggingConfig {
            log_file_name: String::from("test.log"),
            log_level: String::from("info"),
        };

        let guard1 = log_manager
            .init(&logging_config)
            .expect("first init failed");
        let guard2 = log_manager
            .init(&logging_config)
            .expect("second init failed");

        assert!(config_dir.exists(), "log directory should be created");
        drop(guard1);
        drop(guard2);
    }
}
