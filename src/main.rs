use std::{
    io, process,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tucan::{
    core::{
        config::{AppConfig, ConfigManager, LoggingConfig},
        logging::LogManager,
    },
    model::connection::Connection,
    tui::{
        app::{TerminalGuard, TucanApp},
        handler::AppExit,
    },
};

/// Loads application configuration
fn load_app_config(config_manager: &ConfigManager) -> AppConfig {
    match config_manager.load_or_create() {
        Ok(app_config) => app_config,
        Err(err) => {
            // logging is not yet initialized at this point
            eprintln!("[FATAL ERROR] Failed to load configuration file: {:?}", err);
            process::exit(1);
        }
    }
}

/// Initialize logger and logging framework
fn init_logger(logging_config: &LoggingConfig, log_manager: &LogManager) -> WorkerGuard {
    match log_manager.init(logging_config) {
        Ok(log_guard) => log_guard,
        Err(err) => {
            eprintln!("[FATAL ERROR] Failed to initialize logger: {:?}", err);
            process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // prepare the config manager
    let mut config_manager = Arc::new(ConfigManager::default());
    // try to load the configuration
    let app_config = load_app_config(&config_manager);
    // prepare the log manager
    let log_manager = LogManager::new(config_manager.clone());

    // init/setup the logging
    let _log_guard = init_logger(&app_config.logging_config, &log_manager);

    info!("Starting the Tucan application.");

    // color_eyre::install()?; // install error handler for pretty error reports
    enable_raw_mode()?; // in raw mode input is sent directly to the app (no line buffering, no echo)
    let _terminal_guard = TerminalGuard; // terminal guard to ensure terminal restoration

    // prepare the terminal and backend
    let mut stdout = io::stdout(); // get handler to STDOUT
    // switch terminal to alternate screen and enable mouse capture,
    // so UI won't overwrite the shell
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    // from STDOUT prepare the backend and also the terminal abstraction
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create the instance of the app and run the event loop
    let mut app_config = Arc::new(Mutex::new(app_config));
    let mut res: Option<Result<AppExit>> = None;
    loop {
        match TucanApp::new(app_config.clone())
            .await?
            .run_app(&mut terminal)
        {
            Ok(AppExit::Quit) => break,
            Ok(AppExit::Restart) => {
                debug!("Restarting the application");
                // just reload the app config (with potential changes) and let to recreate the app instance
                Connection::reset_db_pool_for_tests();
                config_manager = Arc::new(ConfigManager::default());
                app_config = Arc::new(Mutex::new(load_app_config(&config_manager)));
            }
            Err(error) => res = Some(Err(error)),
            _ => {}
        }
    }

    // return the terminal to the state before,
    // so it will keep running normally as before
    disable_raw_mode()?;
    // return back to the normal screen, disable mouse capture
    // show the cursor (it's disabled during the run of TUI app)
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Some(Err(err)) = res {
        eprintln!("{:?}", err);
    }

    debug!("Closing the Tucan application");
    Ok(())
}
