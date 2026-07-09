use anyhow::Error;

/// `AppEvent` enum represents all the application events/messages
/// used for async communication between the main terminal/rendering loop
/// and any async task that is offloaded to another thread -> I/O task, network calls etc.
#[derive(Debug)]
pub enum AppEvent {
    Config(ConfigEvent),
}

/// `ConfigEvent` enum represent config specific events supported by our application
#[derive(Debug)]
pub enum ConfigEvent {
    ConfigPersisted,
    Error(Error),
}
