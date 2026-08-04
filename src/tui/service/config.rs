use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use tokio::{sync::mpsc::Sender, task::JoinHandle};
use tracing::{debug, error};

use crate::{
    core::config::{AppConfig, ConfigManager},
    tui::event::events::{AppEvent, ConfigEvent},
};

#[derive(Debug)]
pub struct ConfigService {
    config_manager: Arc<ConfigManager>,
    sender: Sender<AppEvent>,
    is_requested_restart: bool,
}

impl ConfigService {
    pub fn new(config_manager: Arc<ConfigManager>, sender: Sender<AppEvent>) -> Self {
        Self {
            config_manager,
            sender,
            is_requested_restart: false,
        }
    }

    pub fn requested_restart(&self) -> bool {
        self.is_requested_restart
    }

    pub fn request_restart(&mut self, restart: bool) {
        self.is_requested_restart = restart;
    }

    pub fn get_config_file_path(&self) -> PathBuf {
        self.config_manager.get_config_file_path()
    }

    pub fn save_config(&self, app_config: &AppConfig) -> JoinHandle<()> {
        let config_manager = self.config_manager.clone();
        let app_config = app_config.clone();
        let sender = self.sender.clone();
        
        tokio::spawn(async move {
            // persist the config using a blocking task to avoid occupying Tokio worker threads
            let result =
                tokio::task::spawn_blocking(move || config_manager.save_config(&app_config))
                    .await
                    .map_err(|err| anyhow!("Config save task panicked: {err:?}"));

            match result {
                Ok(Ok(())) => {
                    match sender
                        .send(AppEvent::Config(ConfigEvent::ConfigPersisted))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful persistence of app config was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about persistence of app config, error: {error}"
                            );
                        }
                    }
                }
                Ok(Err(error)) | Err(error) => {
                    match sender
                        .send(AppEvent::Config(ConfigEvent::Error(error)))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while persisting the app config was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of persisting the app config, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn new_sets_requested_restart_false_and_returns_config_file_path() {
        let dir = tempdir().expect("failed to create temp dir");
        let manager = ConfigManager::new(Some(dir.path().to_path_buf()));
        let (sender, _receiver) = mpsc::channel(1);

        let service = ConfigService::new(Arc::new(manager), sender);

        assert!(!service.requested_restart());
        assert_eq!(
            service.get_config_file_path(),
            service.config_manager.get_config_file_path()
        );

        let mut service = service;
        service.request_restart(true);
        assert!(service.requested_restart());

        service.request_restart(false);
        assert!(!service.requested_restart());
    }

    #[tokio::test]
    async fn save_config_sends_config_persisted_event_on_success() {
        let dir = tempdir().expect("failed to create temp dir");
        let manager = ConfigManager::new(Some(dir.path().to_path_buf()));
        let (sender, mut receiver) = mpsc::channel(1);
        let service = ConfigService::new(Arc::new(manager), sender);

        let config = AppConfig::default();
        let handle = service.save_config(&config);

        handle.await.expect("save_config task panicked");

        let event = receiver
            .recv()
            .await
            .expect("expected a config event to be received");

        match event {
            AppEvent::Config(ConfigEvent::ConfigPersisted) => {}
            other => panic!("unexpected event: {other:?}"),
        }

        assert!(service.get_config_file_path().exists());
    }

    #[tokio::test]
    async fn save_config_sends_error_event_on_failure() {
        let dir = tempdir().expect("failed to create temp dir");
        let manager = ConfigManager::new(Some(dir.path().join("missing")));
        let (sender, mut receiver) = mpsc::channel(1);
        let service = ConfigService::new(Arc::new(manager), sender);

        let config = AppConfig::default();
        let handle = service.save_config(&config);

        handle.await.expect("save_config task panicked");

        let event = receiver
            .recv()
            .await
            .expect("expected an error event to be received");

        match event {
            AppEvent::Config(ConfigEvent::Error(_)) => {}
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn save_config_does_not_panic_when_sender_is_closed() {
        let dir = tempdir().expect("failed to create temp dir");
        let manager = ConfigManager::new(Some(dir.path().to_path_buf()));
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);
        let service = ConfigService::new(Arc::new(manager), sender);

        let config = AppConfig::default();
        let handle = service.save_config(&config);

        handle.await.expect("save_config task panicked");
        assert!(service.get_config_file_path().exists());
    }

    #[tokio::test]
    async fn save_config_does_not_panic_when_sender_is_closed_on_failure() {
        let dir = tempdir().expect("failed to create temp dir");
        let manager = ConfigManager::new(Some(dir.path().join("missing")));
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);
        let service = ConfigService::new(Arc::new(manager), sender);

        let config = AppConfig::default();
        let handle = service.save_config(&config);

        handle.await.expect("save_config task panicked");
        assert!(!service.get_config_file_path().exists());
    }
}
