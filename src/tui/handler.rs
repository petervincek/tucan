use std::sync::{Arc, Mutex};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{Frame, widgets::Paragraph};
use sqlx::{Pool, Sqlite};

use crate::{core::config::AppConfig, model::connection::Connection};

/// `AppPage` enum represents all the application state (pages)
/// that can be rendered and are supported, idea is to use this as
/// a way to navigate between different TUI pages and features
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppPage {
    Menu, // entry point od the application, option to go to any other state
}

/// `Quit` is just simple enum used for signalling the total quit of the application
#[derive(Debug, PartialEq, Eq)]
pub enum AppExit {
    Quit,
    Restart,
    KeepRunning,
}

/// `Handler` is the main application handler data structure that is responsible for:
/// - creating the TUI pages, backend services, TUI components and layouts and wiring them together
/// - exposing functionality to render the TUI pages and components and managing the application state
/// - exposing functionality to delegate and handle terminal events (key events etc.)
/// - exposing functionality to delegate and handle custom events from backend services
pub struct Handler {
    app_config: Arc<Mutex<AppConfig>>,
    db_connection_pool: Arc<Pool<Sqlite>>,
}

impl Handler {
    /// creates the TUI pages and components, backend services and wires them together as a part of `Handler`
    /// manages the application state and the state of the navigation
    pub async fn new(app_config: Arc<Mutex<AppConfig>>) -> Result<Self> {
        let db_connection_pool = Connection::new(app_config.clone())
            .get_db_connection_pool()
            .await?;
        Ok(Self {
            app_config,
            db_connection_pool,
        })
    }

    /// `render_app` will be responsible for rendering the actual application state according to data
    /// and the state of navigation of this handler instance
    pub fn render_app(&mut self, f: &mut Frame<'_>) {
        let main_area = f.area();
        let paragraph_placeholder = Paragraph::new(String::from("Tucan app placeholder"));
        f.render_widget(paragraph_placeholder, main_area);
    }

    /// terminal input events handler for all the key and mouse events coming from terminal interaction
    /// handler will handle the event according to the current application state (which page and state the app
    /// currently is in)
    pub fn handle_term_events(&mut self) -> Result<AppExit> {
        // pool for event every 100ms
        if event::poll(std::time::Duration::from_millis(100))? {
            let current_event = event::read()?;
            if let Event::Key(key_event) = current_event {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::NONE) => return Ok(AppExit::Quit),
                    _ => {}
                }
            }
        }
        Ok(AppExit::KeepRunning)
    }

    /// reads and dispatches all the received events to their registered handlers
    pub fn handle_bus_events(&self) {}
}
