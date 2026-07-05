use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use ratatui::{Frame, Terminal, backend::Backend};

use crate::{
    core::config::AppConfig,
    tui::handler::{AppExit, Handler},
};

/// RAII guard to ensure clean up and restoration of the terminal
/// even in cases of early, unexpected and panic outcomes
pub struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = crossterm::execute!(
            stdout,
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        );
    }
}

/// `TucanApp` represents the main entry point structure for this application
/// it mostly delegates the functionality to the underlying `Handler` abstraction
pub struct TucanApp {
    app_handler: Handler,
}

impl TucanApp {
    /// crates the main application instance
    pub async fn new(app_config: Arc<Mutex<AppConfig>>) -> Result<Self> {
        // create all the dependencies and wire them
        // create the main application handler that encapsulates everything
        let handler = Handler::new(app_config).await?;
        Ok(Self {
            app_handler: handler,
        })
    }

    /// runs the main application loop responsible for rendering widgets/components,
    /// handling user's events from the terminal like key and mouse events and
    /// handling application wide events from async services and method calls
    pub fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<AppExit> {
        // main event/render loop
        loop {
            // 1. render the TUI widgets/components
            terminal
                .draw(|f| {
                    // closure with frame
                    self.render_widgets(f);
                })
                .map_err(|err| anyhow!("{}", err))?;

            // 2. handle the terminal input events like key and mouse events
            match self.handle_term_events()? {
                AppExit::Quit => return Ok(AppExit::Quit),
                AppExit::Restart => return Ok(AppExit::Restart),
                AppExit::KeepRunning => {
                    // do nothing in this case
                }
            }
            // 3. handle the application wide events coming from async services and method/function calls
            self.app_handler.handle_bus_events();
        }
    }

    /// rendering widgets function based on the selected/current page (according to navigation)
    fn render_widgets(&mut self, f: &mut Frame<'_>) {
        // just delegate to application handler
        self.app_handler.render_app(f);
    }

    /// terminal input events handler for all the key and mouse events coming from terminal interaction
    fn handle_term_events(&mut self) -> Result<AppExit> {
        self.app_handler.handle_term_events()
    }
}
