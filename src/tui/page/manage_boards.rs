use std::{
    ops::ControlFlow,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};
use tracing::debug;

use crate::{
    core::config::{AppConfig, KanbanBoard},
    tui::{
        components::text_input::TextInput,
        handler::{
            AppExit::{self},
            include_delegated_event_controls,
        },
        page::common::{Center, EventHandler},
        service::config::ConfigService,
    },
};

/// `CreateBoardFormField` represents the enum of form fields available for widget `CreateBoardForm`
#[derive(Debug)]
enum CreateBoardFormField {
    Id,
    Name,
    Description,
    DbPath,
}

/// `CreateBoardFormState` represents the state for statefull widget `CreateBoardForm`
#[derive(Debug)]
struct CreateBoardFormState {
    id: TextInput,          // widget for capturing the Kanban Board Id
    name: TextInput,        // widget for capturing the Kanban Board Name
    description: TextInput, // widget for capturing the Kanban Board Description
    db_path: TextInput, // widget for capturing the SQLite DB path for data related to Kanban Board
    current_field: CreateBoardFormField,
}

impl CreateBoardFormState {
    pub fn new() -> Self {
        let mut id_text_input = TextInput::new(String::from("")).label(String::from("Board Id:"));
        id_text_input.focused(true);

        let mut name_text_input =
            TextInput::new(String::from("")).label(String::from("Board Name:"));
        name_text_input.focused(false);

        let mut description_text_input =
            TextInput::new(String::from("")).label(String::from("Description:"));
        description_text_input.focused(false);

        let mut db_path_input_text =
            TextInput::new(String::from("")).label(String::from("DB file path:"));
        db_path_input_text.focused(false);

        Self {
            id: id_text_input,
            name: name_text_input,
            description: description_text_input,
            db_path: db_path_input_text,
            current_field: CreateBoardFormField::Id,
        }
    }

    pub fn clear(&mut self) {
        self.id.clear();
        self.name.clear();
        self.description.clear();
        self.db_path.clear();
        self.current_field = CreateBoardFormField::Id;
    }
}

impl EventHandler<(String, String, String, PathBuf), ()> for CreateBoardFormState {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            String::from("Move To Next Field"),
        ));
        match self.current_field {
            CreateBoardFormField::Id => {
                include_delegated_event_controls(&mut event_controls, self.id.get_event_controls());
            }
            CreateBoardFormField::Name => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.name.get_event_controls(),
                );
            }
            CreateBoardFormField::Description => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.description.get_event_controls(),
                );
            }
            CreateBoardFormField::DbPath => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.db_path.get_event_controls(),
                );
            }
        }
        event_controls
    }

    fn handle_event(
        &mut self,
        event: Event,
    ) -> Result<ControlFlow<(String, String, String, PathBuf), ()>> {
        if let Event::Key(key_event) = event
            && (key_event.code == KeyCode::Enter && key_event.modifiers == KeyModifiers::NONE)
            && (self.id.get_buffer() != "")
            && (self.name.get_buffer() != "")
            && (self.description.get_buffer() != "")
            && (self.db_path.get_buffer() != "")
        {
            let board_id = self.id.get_buffer();
            let board_name = self.name.get_buffer();
            let board_description = self.description.get_buffer();
            let db_path = PathBuf::from(self.db_path.get_buffer());
            self.clear();
            return Ok(ControlFlow::Break((
                board_id,
                board_name,
                board_description,
                db_path,
            )));
        } else {
            match self.current_field {
                CreateBoardFormField::Id => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.id.focused(false);
                                self.name.focused(true);
                                self.current_field = CreateBoardFormField::Name;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let id_input_text = &mut self.id;
                                let _result = id_input_text.handle_event(event)?;
                            }
                        }
                    }
                }
                CreateBoardFormField::Name => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.name.focused(false);
                                self.description.focused(true);
                                self.current_field = CreateBoardFormField::Description;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let name_input_text = &mut self.name;
                                let _result = name_input_text.handle_event(event)?;
                            }
                        }
                    }
                }
                CreateBoardFormField::Description => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.description.focused(false);
                                self.db_path.focused(true);
                                self.current_field = CreateBoardFormField::DbPath;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let description_input_text = &mut self.description;
                                let _result = description_input_text.handle_event(event)?;
                            }
                        }
                    }
                }
                CreateBoardFormField::DbPath => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.db_path.focused(false);
                                self.id.focused(true);
                                self.current_field = CreateBoardFormField::Id;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let db_path_input_text = &mut self.db_path;
                                let _result = db_path_input_text.handle_event(event)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(ControlFlow::Continue(()))
    }
}

/// `CreateBoardForm` represents the statefull widget `CreateBoardForm`
#[derive(Debug)]
struct CreateBoardForm {}

impl CreateBoardForm {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for CreateBoardForm {
    type State = CreateBoardFormState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        // prepare the layout
        let vertical_layout = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ]);
        let [
            id_field_area,
            name_field_area,
            description_field_area,
            db_path_field_area,
        ] = area.layout(&vertical_layout);

        // render the id text input
        let id_text_input = &state.id;

        Widget::render(
            id_text_input,
            Center::builder(id_field_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );

        // render the name text input
        let name_text_input = &state.name;

        Widget::render(
            name_text_input,
            Center::builder(name_field_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );

        // render the description text input
        let description_text_input = &state.description;
        Widget::render(
            description_text_input,
            Center::builder(description_field_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );

        // render the db path text input
        let db_path_text_input = &state.db_path;
        Widget::render(
            db_path_text_input,
            Center::builder(db_path_field_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );
    }
}

/// `PageView` represents the specific view page (it's part of navigation) for `ManageBoards` page
/// according to the current application state
#[derive(Debug)]
enum PageView {
    BoardList,
    CreateBoard,
}

/// `ManageBoardsState` represents the state for statefull widget `ManageBoards`
#[derive(Debug)]
pub struct ManageBoardsState {
    app_config: Arc<Mutex<AppConfig>>,
    config_service: Arc<Mutex<ConfigService>>,
    kanban_boards: Vec<(String, KanbanBoard)>,
    page_view: PageView,
    board_list_state: ListState,
    create_board_form_state: CreateBoardFormState,
}

impl ManageBoardsState {
    pub fn new(
        app_config: Arc<Mutex<AppConfig>>,
        config_service: Arc<Mutex<ConfigService>>,
    ) -> Self {
        let mut kanban_boards: Vec<(String, KanbanBoard)> = {
            let boards = &app_config.lock().unwrap().kanban_boards;
            boards
                .iter()
                .map(|(id, kanban_board)| (id.clone(), kanban_board.clone()))
                .collect()
        };
        kanban_boards.sort_by_key(|item| item.0.clone());

        Self {
            app_config,
            config_service,
            kanban_boards,
            page_view: PageView::BoardList,
            board_list_state: ListState::default(),
            create_board_form_state: CreateBoardFormState::new(),
        }
    }
}

impl EventHandler<(), AppExit> for &mut ManageBoardsState {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        match self.page_view {
            PageView::BoardList => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL)),
                    String::from("Create New Board"),
                ));
            }
            PageView::CreateBoard => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back to Board List"),
                ));
                include_delegated_event_controls(
                    &mut event_controls,
                    self.create_board_form_state.get_event_controls(),
                );
            }
        }
        event_controls
    }

    fn handle_event(&mut self, event: Event) -> Result<ControlFlow<(), AppExit>> {
        match self.page_view {
            PageView::BoardList => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                            debug!("Handling the create board");
                            self.page_view = PageView::CreateBoard;
                        }
                        (KeyCode::Up, KeyModifiers::NONE) => {
                            let board_list_state = &mut self.board_list_state;
                            board_list_state.select_previous();
                        }
                        (KeyCode::Down, KeyModifiers::NONE) => {
                            let board_list_state = &mut self.board_list_state;
                            board_list_state.select_next();
                        }
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            let board_list_state = &mut self.board_list_state;
                            let selected_index = board_list_state.selected();
                            if let Some(selected_index) = selected_index {
                                let (id, kanban_board) = &self.kanban_boards[selected_index];
                                let board_name = &kanban_board.name;
                                debug!("Picking board id: {id}, name: {board_name}");
                                let config = &mut self.app_config.lock().unwrap();
                                config.current_board = id.clone();
                                let config_service = &mut self.config_service.lock().unwrap();
                                config_service.save_config(config);
                                return Ok(ControlFlow::Continue(AppExit::KeepRunning));
                            }
                        }
                        _ => {}
                    }
                }
            }
            PageView::CreateBoard => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => self.page_view = PageView::BoardList,
                        // delegate any other event to the create board form state
                        _ => {
                            let result = self.create_board_form_state.handle_event(event)?;
                            if let ControlFlow::Break((
                                board_id,
                                board_name,
                                board_description,
                                db_path,
                            )) = result
                            {
                                // process the collected data from the form and return
                                debug!(
                                    "Updating the app config with board: {board_id} - {board_name}, description: {board_description}, db_path: {db_path:?}"
                                );
                                let config = &mut self.app_config.lock().unwrap();
                                config.kanban_boards.insert(
                                    board_id,
                                    KanbanBoard::new(board_name, board_description, db_path),
                                );
                                // asynchronously save the config file
                                let config_service = &mut self.config_service.lock().unwrap();
                                config_service.save_config(config);
                                return Ok(ControlFlow::Continue(AppExit::KeepRunning));
                            }
                        }
                    }
                }
            }
        }
        Ok(ControlFlow::Continue(AppExit::KeepRunning))
    }
}

/// `ManageBoards` represents the statefull widget for a page responsible for:
/// - creating a new Kanban board (creating new db SQLite file)
/// - changing the current Kanban board, which will allow the user to work with the
/// changed db (Kanban Board)
#[derive(Debug)]
pub struct ManageBoards {
    app_config: Arc<Mutex<AppConfig>>,
}

impl ManageBoards {
    pub fn new(app_config: Arc<Mutex<AppConfig>>) -> Self {
        Self { app_config }
    }
}

impl StatefulWidget for ManageBoards {
    type State = ManageBoardsState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match state.page_view {
            PageView::BoardList => {
                // render the board list
                let currently_active_board =
                    { self.app_config.lock().unwrap().current_board.clone() };
                let boards = &state.kanban_boards;
                let list_items: Vec<ListItem> = boards
                    .iter()
                    .enumerate()
                    .map(|(index, (board_id, board))| {
                        let board_name = &board.name;
                        let text_style = {
                            if currently_active_board == board_id.clone() {
                                Style::default().fg(Color::LightGreen).bold()
                            } else {
                                Style::default().fg(Color::Gray)
                            }
                        };
                        let item_text =
                            Text::from(format!("{index:<3} {board_id:<20} {board_name}"))
                                .style(text_style);
                        ListItem::new(item_text)
                    })
                    .collect();
                let list = List::new(list_items)
                    .highlight_symbol(">> ")
                    .highlight_style(Style::default().fg(Color::LightYellow).bg(Color::DarkGray))
                    .block(
                        Block::default()
                            .borders(Borders::all())
                            .border_style(Style::default().fg(Color::Gray))
                            .title(format!(
                                "Available Boards (active -> {currently_active_board}):"
                            ))
                            .title_style(Style::default().fg(Color::Gray).bold()),
                    );
                let list_state = &mut state.board_list_state;
                StatefulWidget::render(
                    list,
                    Center::builder(area)
                        .horizontally(true)
                        .horizontal_constaint(Constraint::Percentage(95))
                        .vertically(false)
                        .build()
                        .center(),
                    buf,
                    list_state,
                );
            }
            PageView::CreateBoard => {
                // render the create board form
                let create_board_form = CreateBoardForm::new();
                let create_board_form_state = &mut state.create_board_form_state;
                StatefulWidget::render(create_board_form, area, buf, create_board_form_state);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ConfigManager;
    use crate::core::event_bus::EventBus;
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};
    use crate::tui::event::events::{AppEvent, ConfigEvent};
    use crate::tui::page::common::EventHandler;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{buffer::Buffer, layout::Rect};
    use std::{fs, ops::ControlFlow, path::PathBuf, sync::Arc};
    use tempfile::tempdir;

    fn temp_config_service(
        event_bus: &EventBus<AppEvent>,
    ) -> (
        tempfile::TempDir,
        Arc<Mutex<ConfigService>>,
        impl FnOnce(Box<dyn Fn(AppEvent) -> Result<()> + Send + Sync>) + Send + 'static,
    ) {
        let dir = tempdir().expect("failed to create temp dir");
        let manager = ConfigManager::new(Some(dir.path().to_path_buf()));
        let (sender, register) = event_bus.create_channel();
        let config_service = ConfigService::new(Arc::new(manager), sender);
        (dir, Arc::new(Mutex::new(config_service)), register)
    }

    fn make_test_app_config() -> Arc<Mutex<AppConfig>> {
        let mut config = AppConfig::default();
        config.current_board = String::from("board-1");
        config.kanban_boards.insert(
            String::from("board-1"),
            KanbanBoard::new(
                String::from("First Board"),
                String::from("The first test board"),
                PathBuf::from("board1.db"),
            ),
        );
        config.kanban_boards.insert(
            String::from("board-2"),
            KanbanBoard::new(
                String::from("Second Board"),
                String::from("The second test board"),
                PathBuf::from("board2.db"),
            ),
        );
        Arc::new(Mutex::new(config))
    }

    fn fill_text_input(input: &mut TextInput, text: &str) {
        for ch in text.chars() {
            let _ = input
                .handle_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(ch),
                    KeyModifiers::NONE,
                )))
                .unwrap();
        }
    }

    #[test]
    fn test_create_board_form_state_tab_moves_focus_to_next_field() {
        let mut form_state = CreateBoardFormState::new();
        assert!(form_state.id.is_focused());

        let _ = form_state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();

        assert!(!form_state.id.is_focused());
        assert!(form_state.name.is_focused());
    }

    #[test]
    fn test_create_board_form_state_tab_cycles_through_all_fields() {
        let mut form_state = CreateBoardFormState::new();

        let _ = form_state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(form_state.name.is_focused());

        let _ = form_state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(form_state.description.is_focused());

        let _ = form_state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(form_state.db_path.is_focused());

        let _ = form_state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(form_state.id.is_focused());
    }

    #[test]
    fn test_create_board_form_state_submits_when_all_fields_are_populated() {
        let mut form_state = CreateBoardFormState::new();
        fill_text_input(&mut form_state.id, "new-board");
        fill_text_input(&mut form_state.name, "New Board");
        fill_text_input(&mut form_state.description, "A new board description");
        fill_text_input(&mut form_state.db_path, "/tmp/new-board.db");

        let result = form_state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        match result {
            ControlFlow::Break((id, name, description, db_path)) => {
                assert_eq!(id, "new-board");
                assert_eq!(name, "New Board");
                assert_eq!(description, "A new board description");
                assert_eq!(db_path, PathBuf::from("/tmp/new-board.db"));
            }
            _ => panic!("Expected form submission to break with filled values"),
        }
    }

    #[test]
    fn test_create_board_form_state_does_not_submit_when_missing_values() {
        let mut form_state = CreateBoardFormState::new();
        fill_text_input(&mut form_state.id, "new-board");
        fill_text_input(&mut form_state.name, "New Board");
        fill_text_input(&mut form_state.description, "A new board description");

        let result = form_state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(result, ControlFlow::Continue(())));
    }

    #[test]
    fn test_create_board_form_state_clear_resets_fields_and_current_field() {
        let mut form_state = CreateBoardFormState::new();
        fill_text_input(&mut form_state.id, "id");
        fill_text_input(&mut form_state.name, "name");
        fill_text_input(&mut form_state.description, "description");
        fill_text_input(&mut form_state.db_path, "/tmp/db.db");
        form_state.current_field = CreateBoardFormField::DbPath;

        form_state.clear();

        assert_eq!(form_state.id.get_buffer(), "");
        assert_eq!(form_state.name.get_buffer(), "");
        assert_eq!(form_state.description.get_buffer(), "");
        assert_eq!(form_state.db_path.get_buffer(), "");
        assert!(matches!(form_state.current_field, CreateBoardFormField::Id));
    }

    #[test]
    fn test_manage_boards_state_handle_event_ctrl_n_enters_create_board_page() {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, _register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config, config_service.clone());

        let mut state_ref = &mut state;
        let _ = state_ref
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state_ref.page_view, PageView::CreateBoard));
        let controls = state_ref.get_event_controls();
        assert!(
            controls
                .iter()
                .any(|(_, description)| description == "Go Back to Board List")
        );
    }

    #[test]
    fn test_manage_boards_state_list_navigation_moves_selection_up_and_down() {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, _register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config, config_service.clone());

        let mut state_ref = &mut state;
        let _ = state_ref
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state_ref.board_list_state.selected(), Some(0));

        let _ = state_ref
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state_ref.board_list_state.selected(), Some(1));

        let _ = state_ref
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state_ref.board_list_state.selected(), Some(0));
    }

    #[test]
    fn test_manage_boards_state_create_board_page_esc_returns_to_board_list() {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, _register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config, config_service.clone());
        state.page_view = PageView::CreateBoard;

        let mut state_ref = &mut state;
        let _ = state_ref
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();

        assert!(matches!(state_ref.page_view, PageView::BoardList));
    }

    #[test]
    fn test_manage_boards_state_get_event_controls_in_create_board_page_includes_delegated_controls()
     {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, _register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config, config_service.clone());
        state.page_view = PageView::CreateBoard;

        let state_ref = &mut state;
        let controls = state_ref.get_event_controls();

        assert!(
            controls
                .iter()
                .any(|(_, description)| description == "Go Back to Board List")
        );
        assert!(
            controls
                .iter()
                .any(|(_, description)| description == "Move To Next Field")
        );
    }

    #[test]
    fn test_manage_boards_state_get_event_controls_in_board_list_page_includes_create_new_board() {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, _register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config, config_service.clone());

        let state_ref = &mut state;
        let controls = state_ref.get_event_controls();

        assert!(
            controls
                .iter()
                .any(|(_, description)| description == "Create New Board")
        );
    }

    #[tokio::test]
    async fn test_manage_boards_state_selects_board_on_enter_and_persists_config() {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config.clone(), config_service.clone());

        state.board_list_state.select(Some(1));

        let mut state_ref = &mut state;
        let result = state_ref
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(
            result,
            ControlFlow::Continue(AppExit::KeepRunning)
        ));
        let locked = app_config.lock().unwrap();
        assert_eq!(locked.current_board, "board-2");

        register(Box::new({
            let config_service = config_service.clone();
            move |app_event| {
                match app_event {
                    AppEvent::Config(ConfigEvent::ConfigPersisted) => {
                        let config_service = config_service.lock().unwrap();
                        let persisted = fs::read_to_string(config_service.get_config_file_path())
                            .expect("failed to read persisted config");
                        assert!(persisted.contains("current_board = \"board-2\""));
                    }
                    AppEvent::Config(ConfigEvent::Error(_error)) => {
                        panic!("expecting no error");
                    }
                }
                Ok(())
            }
        }));
    }

    #[tokio::test]
    async fn test_manage_boards_state_add_new_board_persists_it_in_config() {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config.clone(), config_service.clone());
        state.page_view = PageView::CreateBoard;

        fill_text_input(&mut state.create_board_form_state.id, "generated-board");
        fill_text_input(&mut state.create_board_form_state.name, "Generated Board");
        fill_text_input(
            &mut state.create_board_form_state.description,
            "Created during test",
        );
        fill_text_input(
            &mut state.create_board_form_state.db_path,
            "/tmp/generated.db",
        );

        let mut state_ref = &mut state;
        let result = state_ref
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(
            result,
            ControlFlow::Continue(AppExit::KeepRunning)
        ));
        let locked = app_config.lock().unwrap();
        let board = locked
            .kanban_boards
            .get("generated-board")
            .expect("new board should be present");

        assert_eq!(board.name, "Generated Board");
        assert_eq!(board.description, "Created during test");
        assert_eq!(board.db_url, PathBuf::from("/tmp/generated.db"));

        register(Box::new({
            let config_service = config_service.clone();
            move |app_event| {
                match app_event {
                    AppEvent::Config(ConfigEvent::ConfigPersisted) => {
                        let config_service = config_service.lock().unwrap();
                        let persisted = fs::read_to_string(config_service.get_config_file_path())
                            .expect("failed to read persisted config");
                        assert!(persisted.contains("generated-board"));
                    }
                    AppEvent::Config(ConfigEvent::Error(_error)) => {
                        panic!("expecting no error");
                    }
                }
                Ok(())
            }
        }));
    }

    #[test]
    fn test_manage_boards_render_board_list_contains_titles_and_items() {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, _register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config.clone(), config_service.clone());

        let widget = ManageBoards::new(app_config.clone());
        let area = Rect::new(0, 0, 50, 8);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);

        assert!(rendered.contains("Available Boards (active -> board-1):"));
        assert!(rendered.contains("board-1"));
        assert!(rendered.contains("board-2"));
    }

    #[test]
    fn test_manage_boards_render_create_board_form_shows_all_input_fields() {
        let event_bust = EventBus::<AppEvent>::new();
        let app_config = make_test_app_config();
        let (_dir, config_service, _register) = temp_config_service(&event_bust);
        let mut state = ManageBoardsState::new(app_config.clone(), config_service.clone());
        state.page_view = PageView::CreateBoard;

        let widget = ManageBoards::new(app_config.clone());
        let area = Rect::new(0, 0, 60, 14);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);

        assert!(rendered.contains("Board Id:"));
        assert!(rendered.contains("Board Name:"));
        assert!(rendered.contains("Description:"));
        assert!(rendered.contains("DB file path:"));
    }

    #[test]
    fn test_manage_boards_create_board_form_render_output() {
        let mut form_state = CreateBoardFormState::new();
        let widget = CreateBoardForm::new();
        let area = Rect::new(0, 0, 50, 14);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut form_state);
        let rendered = buffer_to_string(&buf);

        let expected_output = "
┌Board Id:───────────────────────────────────────┐
│|                                               │
└────────────────────────────────────────────────┘
┌Board Name:─────────────────────────────────────┐
│|                                               │
└────────────────────────────────────────────────┘
┌Description:────────────────────────────────────┐
│|                                               │
└────────────────────────────────────────────────┘
┌DB file path:───────────────────────────────────┐
│|                                               │
└────────────────────────────────────────────────┘
                                                  
                                                  ";
        assert_rendered_output(&rendered, expected_output);
    }
}
