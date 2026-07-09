use std::{
    ops::ControlFlow,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
    style::Style,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{
    core::config::AppConfig,
    tui::{page::common::EventHandler, service::config::ConfigService},
};

/// `ManageBoardDetailsState` represents the state for statefull widget `ManageBoardDetails`
pub struct ManageBoardDetailsState {}

impl ManageBoardDetailsState {
    pub fn new() -> Self {
        Self {}
    }
}

impl EventHandler<(), ()> for ManageBoardDetailsState {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        event_controls
    }

    fn handle_event(&mut self, event: Event) -> Result<ControlFlow<(), ()>> {
        Ok(std::ops::ControlFlow::Continue(()))
    }
}

/// `ManageBoardDetails` represents the statefull widget for a page responsible for:
/// - managing the cards of specific Kanban board and columns of Kanban Board
pub struct ManageBoardDetails {
    app_config: Arc<Mutex<AppConfig>>,
    config_service: Arc<Mutex<ConfigService>>,
}

impl ManageBoardDetails {
    pub fn new(
        app_config: Arc<Mutex<AppConfig>>,
        config_service: Arc<Mutex<ConfigService>>,
    ) -> Self {
        Self {
            app_config,
            config_service,
        }
    }
}

impl StatefulWidget for ManageBoardDetails {
    type State = ManageBoardDetailsState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        // get currently active board
        let active_board = { self.app_config.lock().unwrap().current_board.clone() };
        let board_placeholder = Paragraph::new(format!("Manage Board Details - {active_board}"))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ratatui::style::Color::Gray))
                    .title("Manage Board Details")
                    .title_style(Style::default().bold()),
            );
        Widget::render(board_placeholder, area, buf);
    }
}
