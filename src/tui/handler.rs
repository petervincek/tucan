use std::{
    collections::HashSet,
    ops::ControlFlow,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};
use textwrap::Options;
use tracing::debug;

use crate::{
    core::{
        config::{AppConfig, ConfigManager},
        event_bus::EventBus,
    },
    model::{board::BoardColumnRepo, card::CardRepo, connection::Connection},
    tui::{
        components::{
            footer::Footer,
            header::Header,
            notification_panel::{NotificationMessage, NotificationPanel},
        },
        event::events::{AppEvent, ConfigEvent},
        page::{
            common::EventHandler,
            manage_board_details::{ManageBoardDetails, ManageBoardDetailsState},
            manage_boards::{ManageBoards, ManageBoardsState},
        },
        service::{
            board::BoardService, card::CardService, config::ConfigService,
            notifications::NotificationService,
        },
    },
};

type AppEventHandler = Box<dyn Fn(AppEvent) -> Result<()> + Send + Sync>;

/// `AppPage` enum represents all the application state (pages)
/// that can be rendered and are supported, idea is to use this as
/// a way to navigate between different TUI pages and features
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppPage {
    KanbanBoards,       // page for managing all the boards
    KanbanBoardDetails, // page dedicated to specific board
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
    // references to other services
    app_config: Arc<Mutex<AppConfig>>,
    config_service: Arc<Mutex<ConfigService>>,
    event_bus: EventBus<AppEvent>,
    notification_bus: EventBus<NotificationMessage>,
    // inner state of the app
    current_page: AppPage,
    is_notification_panel_visible: bool,
    // some widgets
    notification_panel: Arc<Mutex<NotificationPanel>>,
    // some state for statefull widget
    manage_boards_state: ManageBoardsState,
    manage_board_details_state: Arc<Mutex<ManageBoardDetailsState>>,
}

impl Handler {
    /// creates the TUI pages and components, backend services and wires them together as a part of `Handler`
    /// manages the application state and the state of the navigation
    pub async fn new(app_config: Arc<Mutex<AppConfig>>, config_dir: PathBuf) -> Result<Self> {
        let db_pool = Connection::new(app_config.clone(), config_dir)
            .get_db_connection_pool()
            .await?;
        // create notification bus, notification service, notification panel
        let notification_bus = EventBus::<NotificationMessage>::new();
        let (notification_service_sender, register_event_handler_notification_service) =
            notification_bus.create_channel();
        let notification_service = Arc::new(NotificationService::new(notification_service_sender));
        let notification_panel = Arc::new(Mutex::new(NotificationPanel::new(60)));
        register_event_handler_notification_service(Box::new({
            let notification_panel = Arc::clone(&notification_panel);
            move |notification_message| {
                debug!("Handling notification: {notification_message:?}");
                let panel = &mut *notification_panel.lock().unwrap();
                panel.add_notification(notification_message);
                Ok(())
            }
        }));

        let event_bus = EventBus::<AppEvent>::new();
        let (config_service, register_event_handler_config_service) =
            create_config_service(Arc::new(ConfigManager::default()), &event_bus);
        // register the event handler
        register_event_handler_config_service(Box::new({
            let config_service = config_service.clone();
            let notification_service = notification_service.clone();
            move |app_event| {
                match app_event {
                    AppEvent::Config(ConfigEvent::ConfigPersisted) => {
                        debug!("Setting config service with restart");
                        config_service.lock().unwrap().request_restart(true);
                        notification_service.send_notification(NotificationMessage::InfoMsg(
                            "Config file perstisted, restart of app requested".to_string(),
                            Instant::now(),
                        ));
                    }
                    AppEvent::Config(ConfigEvent::Error(error)) => {
                        debug!("Received error: {error}");
                        notification_service.send_notification(NotificationMessage::ErrorMsg(
                            format!("Error: {error}"),
                            Instant::now(),
                        ));
                    }
                    AppEvent::Board(_) => {}
                    AppEvent::Card(_) => {}
                }
                Ok(())
            }
        }));

        let board_column_repo = Arc::new(BoardColumnRepo::new(db_pool.clone()));
        let (board_service, register_event_handler_board_service) =
            create_board_service(board_column_repo, &event_bus);

        let card_repo = Arc::new(CardRepo::new(db_pool.clone()));
        let (card_service, register_event_handler_card_service) =
            create_card_service(card_repo, &event_bus);

        let manage_board_details_state = Arc::new(Mutex::new(ManageBoardDetailsState::new(
            app_config.clone(),
            board_service,
            card_service,
            notification_service.clone(),
        )));

        register_event_handler_board_service(Box::new({
            let manage_board_details_state = manage_board_details_state.clone();
            move |app_event| {
                let manage_board_details_state = &mut manage_board_details_state.lock().unwrap();
                manage_board_details_state.handle_app_event(app_event);
                Ok(())
            }
        }));

        register_event_handler_card_service(Box::new({
            let manage_board_details_state = manage_board_details_state.clone();
            move |app_event| {
                let manage_board_details_state = &mut manage_board_details_state.lock().unwrap();
                manage_board_details_state.handle_app_event(app_event);
                Ok(())
            }
        }));

        Ok(Self {
            app_config: app_config.clone(),
            config_service: config_service.clone(),
            event_bus,
            notification_bus,
            current_page: AppPage::KanbanBoards,
            is_notification_panel_visible: true,
            notification_panel,
            manage_boards_state: ManageBoardsState::new(app_config, config_service.clone()),
            manage_board_details_state,
        })
    }

    /// `render_app` will be responsible for rendering the actual application state according to data
    /// and the state of navigation of this handler instance
    pub fn render_app(&mut self, f: &mut Frame<'_>) {
        // let's prepare the main layout of the app
        // we will have place for header, footer, notification panel and the dynamic page that will be rendered based
        // on the state of navigation
        let main_area = f.area();

        // create the dynamic layout
        let (first_column_area, notification_panel_area) = {
            if self.is_notification_panel_visible {
                // in this case we need 2 columns layout
                let horizontal_layout =
                    Layout::horizontal([Constraint::Fill(4), Constraint::Fill(1)]);
                let [first_column, second_column] = main_area.layout(&horizontal_layout);
                (first_column, Some(second_column))
            } else {
                // in this case we need just one column layout, which is equel to main_area
                (main_area, None)
            }
        };

        // prepare the layout for the header, dynamic content and footer
        let vertical_layout = Layout::vertical([
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(3),
        ]);
        let [header_area, dynamic_content_area, footer_area] =
            first_column_area.layout(&vertical_layout);

        // render the header
        let header = &mut Header::new(String::from(
            r#" _____ _   _  ___   _   _  _         _____ _   _ ___   _  __          _               
|_   _| | | |/ __| /_\ | \| |  ___  |_   _| | | |_ _| | |/ /__ _ _ _ | |__  __ _ _ _  
  | | | |_| | (__ / _ \| .` | |___|   | | | |_| || |  | ' </ _` | ' \| '_ \/ _` | ' \ 
  |_|  \___/ \___/_/ \_\_|\_|         |_|  \___/|___| |_|\_\__,_|_||_|_.__/\__,_|_||_|"#,
        ));
        f.render_widget(header, header_area);

        // common event controls
        let mut event_controls = Vec::new();
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            String::from("Quit app"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL)),
            String::from("Toggle Notification Panel"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL)),
            String::from("Toggle Active Board"),
        ));

        // render the dynamic content
        match self.current_page {
            AppPage::KanbanBoards => {
                let manage_boards_state = &mut self.manage_boards_state;
                include_delegated_event_controls(
                    &mut event_controls,
                    manage_boards_state.get_event_controls(),
                );
                let manage_boards_page = ManageBoards::new(self.app_config.clone());
                f.render_stateful_widget(
                    manage_boards_page,
                    dynamic_content_area,
                    manage_boards_state,
                );
            }
            AppPage::KanbanBoardDetails => {
                let manage_board_details_state =
                    &mut self.manage_board_details_state.lock().unwrap();
                include_delegated_event_controls(
                    &mut event_controls,
                    manage_board_details_state.get_event_controls(),
                );
                let manage_board_details_page = ManageBoardDetails::new();
                f.render_stateful_widget(
                    manage_board_details_page,
                    dynamic_content_area,
                    manage_board_details_state,
                );
            }
        }

        // render the footer
        let footer = &mut Footer::new(String::from("Footer"));
        render_dynamic_footer(f, footer_area, footer, event_controls);

        // render the optional notification panel
        if let Some(notification_panel_area) = notification_panel_area {
            let notification_panel = &mut *self.notification_panel.lock().unwrap();
            f.render_widget(notification_panel, notification_panel_area);
        }
    }

    /// terminal input events handler for all the key and mouse events coming from terminal interaction
    /// handler will handle the event according to the current application state (which page and state the app
    /// currently is in)
    pub fn handle_term_events(&mut self) -> Result<AppExit> {
        // check for requested restart
        {
            let config_service = self.config_service.lock().unwrap();
            if config_service.requested_restart() {
                return Ok(AppExit::Restart);
            }
        }
        // pool for event every 100ms
        if event::poll(std::time::Duration::from_millis(100))? {
            let current_event = event::read()?;
            // handle just key events
            if let Event::Key(key_event) = current_event {
                let key_code = key_event.code;
                let key_modifiers = key_event.modifiers;
                debug!("Handling delegation of {key_code}, {key_modifiers}");
                // general logic for handling key events regardles the page
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => return Ok(AppExit::Quit),
                    (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                        // toggle the notification panel on and off
                        self.is_notification_panel_visible = !self.is_notification_panel_visible;
                    }
                    (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                        // open active board page
                        if self.current_page == AppPage::KanbanBoards {
                            self.current_page = AppPage::KanbanBoardDetails;
                        } else {
                            self.current_page = AppPage::KanbanBoards
                        }
                    }
                    // any other event pass/delegate to the page/widget handlers
                    _ => {
                        // page specific handlers
                        debug!("Handling delegation ...");
                        match self.current_page {
                            AppPage::KanbanBoards => {
                                let mut manage_boards_state = &mut self.manage_boards_state;
                                let result = manage_boards_state.handle_event(current_event)?;
                                if let ControlFlow::Break(_) = result {
                                    return Ok(AppExit::Restart);
                                }
                            }
                            AppPage::KanbanBoardDetails => {
                                let manage_board_details_state =
                                    &mut self.manage_board_details_state.lock().unwrap();
                                let result =
                                    manage_board_details_state.handle_event(current_event)?;
                                if let ControlFlow::Break(_) = result {
                                    return Ok(AppExit::Restart);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(AppExit::KeepRunning)
    }

    /// reads and dispatches all the received events to their registered handlers
    pub fn handle_bus_events(&self) {
        self.event_bus.try_process();
        self.notification_bus.try_process();
    }
}

/// creates a helper navigation message/instruction for the purpose of being displayed
pub fn create_navigation_message(event_controls: Vec<(Event, String)>) -> String {
    let mut parts = Vec::new();
    for (event, desc) in event_controls {
        let key_str = match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                let mut mod_str = String::new();
                if modifiers.contains(KeyModifiers::CONTROL) {
                    mod_str.push_str("Ctrl+");
                }
                if modifiers.contains(KeyModifiers::ALT) {
                    mod_str.push_str("Alt+");
                }
                if modifiers.contains(KeyModifiers::SHIFT) {
                    mod_str.push_str("Shift+");
                }
                let key = match code {
                    KeyCode::Char(c) => format!("'{}'", c),
                    KeyCode::Enter => "Enter".to_string(),
                    KeyCode::Esc => "Esc".to_string(),
                    KeyCode::Tab => "Tab".to_string(),
                    KeyCode::Backspace => "Backspace".to_string(),
                    KeyCode::Left => "←".to_string(),
                    KeyCode::Right => "→".to_string(),
                    KeyCode::Up => "↑".to_string(),
                    KeyCode::Down => "↓".to_string(),
                    other => format!("{:?}", other),
                };
                format!("{}{}", mod_str, key)
            }
            // You can add more event types if needed
            _ => format!("{:?}", event),
        };
        parts.push(format!("{}: {}", key_str, desc));
    }
    parts.join(" | ")
}

/// updates the footer message and renders it to the provided area
pub fn render_dynamic_footer(
    f: &mut Frame<'_>,
    area: Rect,
    footer: &mut Footer,
    event_controls: Vec<(Event, String)>,
) {
    // prepare dynamic content for the footer
    let footer_msg = create_navigation_message(event_controls);
    let wrapped_footer_msg = textwrap::wrap(
        &footer_msg,
        Options::new(area.width.saturating_sub(4).max(1) as usize),
    )
    .join("\n");
    footer.set_message(wrapped_footer_msg);
    f.render_widget(footer, area);
}

pub fn include_delegated_event_controls(
    event_controls: &mut Vec<(Event, String)>,
    delegated_event_controls: Vec<(Event, String)>,
) {
    let hash_event: HashSet<Event> = event_controls
        .iter()
        .map(|(event, _)| event.clone())
        .collect();
    for (delegated_event, control_msg) in delegated_event_controls {
        if !hash_event.contains(&delegated_event) {
            event_controls.push((delegated_event, control_msg));
        }
    }
}

/// `create_config_service` creates the `ConfigService` with it's own dedicated channel for async
/// communication, returns also the register function allowing to register the event handler for the async
/// communication with the caller (back referencing the caller)
pub fn create_config_service(
    config_manager: Arc<ConfigManager>,
    event_bus: &EventBus<AppEvent>,
) -> (
    Arc<Mutex<ConfigService>>,
    impl FnOnce(AppEventHandler) + Send + 'static,
) {
    let (config_service_sender, register_event_handler_config_service) = event_bus.create_channel();
    let config_service = Arc::new(Mutex::new(ConfigService::new(
        config_manager,
        config_service_sender,
    )));
    (config_service, register_event_handler_config_service)
}

pub fn create_board_service(
    board_column_repo: Arc<BoardColumnRepo>,
    event_bus: &EventBus<AppEvent>,
) -> (
    Arc<BoardService>,
    impl FnOnce(AppEventHandler) + Send + 'static,
) {
    let (board_service_sender, register_event_handler_board_service) = event_bus.create_channel();
    let board_service = Arc::new(BoardService::new(board_column_repo, board_service_sender));
    (board_service, register_event_handler_board_service)
}

pub fn create_card_service(
    card_repo: Arc<CardRepo>,
    event_bus: &EventBus<AppEvent>,
) -> (
    Arc<CardService>,
    impl FnOnce(AppEventHandler) + Send + 'static,
) {
    let (card_service_sender, register_event_handler_card_service) = event_bus.create_channel();
    let card_service = Arc::new(CardService::new(card_repo, card_service_sender));
    (card_service, register_event_handler_card_service)
}
