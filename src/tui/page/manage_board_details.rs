use std::{
    ops::ControlFlow,
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::Result;
use chrono::NaiveDateTime;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};
use tracing::debug;

use crate::{
    core::config::AppConfig,
    model::board::{BoardColumn, NewBoardColumn},
    tui::{
        components::{
            choice_picker::{self, Choice, ChoicePicker, ChoicePickerState},
            integer_input::{IntegerInput, IntegerRange},
            notification_panel::NotificationMessage,
            text_input::TextInput,
        },
        event::events::{AppEvent, BoardEvent},
        handler::include_delegated_event_controls,
        page::common::{Center, EventHandler},
        service::{board::BoardService, notifications::NotificationService},
    },
};

/// `ManageBoardColumFormField` represents the enum of form fields available for widget `ManageBoardColumnForm`
#[derive(Debug, Clone)]
enum ManageBoardColumnFormField {
    Name,
    WipLimit,
    Position,
}

/// `ManageBoardColumnFormState` represents the state for statefull widget `ManageBoardColumnForm`
#[derive(Debug, Clone)]
struct ManageBoardColumnFormState {
    id: Option<String>, // state that will hold the optional id of the board column for already existing data
    // form for update vs. form for create
    name: TextInput,         // widget for capturing the Kanban Board Column Name
    wip_limit: IntegerInput, // widget for capturing the Kanban Board Column's WorkInProgress limit
    position: IntegerInput,  // widget for capturing the Kanban Board Column's position
    current_field: ManageBoardColumnFormField,
}

impl ManageBoardColumnFormState {
    pub fn new() -> Self {
        let mut name_text_input =
            TextInput::new(String::from("")).label(String::from("Board Column Name:"));
        name_text_input.focused(true);

        let wip_limit_integer_input =
            IntegerInput::new(1, Some(IntegerRange::new(1, 100).unwrap()), false)
                .unwrap()
                .label(String::from("WorkInProgress Limit:"));

        let position_integer_input =
            IntegerInput::new(0, Some(IntegerRange::new(0, 100).unwrap()), false)
                .unwrap()
                .label(String::from("Board Column Position:"));

        Self {
            id: None,
            name: name_text_input,
            wip_limit: wip_limit_integer_input,
            position: position_integer_input,
            current_field: ManageBoardColumnFormField::Name,
        }
    }

    pub fn preset_with_board_column_data(&mut self, board_column: BoardColumn) {
        let BoardColumn {
            id,
            name,
            wip_limit,
            position,
            created_at: _,
        } = board_column;
        self.id = Some(id);

        self.name = TextInput::new(name).label(String::from("Board Column Name:"));
        self.name.focused(true);

        self.wip_limit
            .set_input_value(wip_limit.try_into().unwrap())
            .unwrap();
        self.wip_limit.focused(false);

        self.position
            .set_input_value(position.try_into().unwrap())
            .unwrap();
        self.position.focused(false);

        self.current_field = ManageBoardColumnFormField::Name;
    }

    pub fn clear(&mut self) {
        self.id = None;

        self.name.clear();
        self.name.focused(true);

        self.wip_limit.set_input_value(1).unwrap();
        self.wip_limit.focused(false);

        self.position.set_input_value(0).unwrap();
        self.position.focused(false);

        self.current_field = ManageBoardColumnFormField::Name;
    }
}

impl EventHandler<(Option<String>, String, i32, i32), ()> for ManageBoardColumnFormState {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            String::from("Move To Next Field"),
        ));
        match self.current_field {
            ManageBoardColumnFormField::Name => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.name.get_event_controls(),
                );
            }
            ManageBoardColumnFormField::WipLimit => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.wip_limit.get_event_controls(),
                );
            }
            ManageBoardColumnFormField::Position => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.position.get_event_controls(),
                );
            }
        }
        event_controls
    }

    fn handle_event(
        &mut self,
        event: Event,
    ) -> Result<ControlFlow<(Option<String>, String, i32, i32), ()>> {
        if let Event::Key(key_event) = event
            && (key_event.code == KeyCode::Enter && key_event.modifiers == KeyModifiers::NONE)
            && (self.name.get_buffer() != "")
        {
            let board_column_id = self.id.clone();
            let board_column_name = self.name.get_buffer();
            let wip_limit = self.wip_limit.get_input_value();
            let board_column_position = self.position.get_input_value();
            // self.clear();
            return Ok(ControlFlow::Break((
                board_column_id,
                board_column_name,
                wip_limit,
                board_column_position,
            )));
        } else {
            match self.current_field {
                ManageBoardColumnFormField::Name => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.name.focused(false);
                                self.wip_limit.focused(true);
                                self.current_field = ManageBoardColumnFormField::WipLimit;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let name_input_text = &mut self.name;
                                let _result = name_input_text.handle_event(event)?;
                            }
                        }
                    }
                }
                ManageBoardColumnFormField::WipLimit => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.wip_limit.focused(false);
                                self.position.focused(true);
                                self.current_field = ManageBoardColumnFormField::Position;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let wip_limit_integer_input = &mut self.wip_limit;
                                let _result = wip_limit_integer_input.handle_event(event)?;
                            }
                        }
                    }
                }
                ManageBoardColumnFormField::Position => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.position.focused(false);
                                self.name.focused(true);
                                self.current_field = ManageBoardColumnFormField::Name;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let position_integer_input = &mut self.position;
                                let _result = position_integer_input.handle_event(event)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(ControlFlow::Continue(()))
    }
}

/// `ManageBoardColumnForm` represents the statefull widget
#[derive(Debug)]
struct ManageBoardColumnForm {}

impl ManageBoardColumnForm {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for ManageBoardColumnForm {
    type State = ManageBoardColumnFormState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        // prepare the layout
        let vertical_layout = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ]);
        let [
            title_area,
            name_field_area,
            wip_limit_field_area,
            position_field_area,
        ] = area.layout(&vertical_layout);

        // render the title above the form widgets
        let title = if let Some(id) = &state.id {
            Text::from(format!("Update of column (id: {id})"))
                .bold()
                .fg(Color::Green)
        } else {
            Text::from(format!("Create new column"))
                .bold()
                .fg(Color::Green)
        };
        Widget::render(
            title,
            Center::builder(title_area)
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

        // render the wip limit integer input
        let wip_limit_integer_input = &state.wip_limit;
        Widget::render(
            wip_limit_integer_input,
            Center::builder(wip_limit_field_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );

        // render the position integer input
        let position_integer_input = &state.position;
        Widget::render(
            position_integer_input,
            Center::builder(position_field_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );
    }
}

/// `PageView` represents the specific view page (it's part of navigation) for `ManageBoardDetails` page
/// according to the current application state
#[derive(Debug, Clone)]
enum PageView {
    BoardDetails,
    ManageBoardColumn,
    ConfirmDialog,
}

/// `ActionToConfirm` represents all the action on thos page that needs to be confirmed before proceeding further
#[derive(Debug, Clone, PartialEq)]
enum ActionToConfirm {
    NoAction,
    DeleteBoardColumn,
}

/// `ManageBoardDetailsState` represents the state for statefull widget `ManageBoardDetails`
#[derive(Clone)]
pub struct ManageBoardDetailsState {
    app_config: Arc<Mutex<AppConfig>>,
    board_service: Arc<BoardService>,
    notification_service: Arc<NotificationService>,
    // page state
    board_columns: Vec<BoardColumn>,
    current_column_index: usize,
    page_view: PageView,
    current_action_to_confirm: ActionToConfirm,
    manage_board_column_form_state: ManageBoardColumnFormState,
    confirm_dialog_state: (Vec<Choice<bool>>, ChoicePickerState),
}

impl ManageBoardDetailsState {
    fn current_window_start(&self, total_columns_num: usize, display_columns_num: usize) -> usize {
        if total_columns_num <= display_columns_num {
            return 0;
        }

        let half_window = display_columns_num / 2;
        let start = self.current_column_index.saturating_sub(half_window);

        if start + display_columns_num > total_columns_num {
            total_columns_num - display_columns_num
        } else {
            start
        }
    }

    pub fn new(
        app_config: Arc<Mutex<AppConfig>>,
        board_service: Arc<BoardService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        // trigger the board columns fetching, fire-and-forget call
        board_service.list_columns();
        Self {
            app_config,
            board_service,
            notification_service,
            board_columns: vec![],
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        }
    }

    pub fn handle_app_event(&mut self, app_event: AppEvent) {
        match app_event {
            AppEvent::Board(BoardEvent::BoardColumnCreated(created_board_column)) => {
                debug!("New Board Column created: {created_board_column:?}");
                let board_column_name = created_board_column.name;
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("New Board Column created, name: {board_column_name}"),
                        Instant::now(),
                    ));
                // load the board columns to reflect the changes
                self.board_service.list_columns();
            }
            AppEvent::Board(BoardEvent::BoardColumnFetched(fetched_board_column)) => {
                debug!("Board Column fetched: {fetched_board_column:?}");
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Board Column fetched: {fetched_board_column:?}"),
                        Instant::now(),
                    ));
            }
            AppEvent::Board(BoardEvent::BoardColumnsFetched(fetched_board_columns)) => {
                debug!("Board Columns fetched: {fetched_board_columns:?}");
                let board_columns_size = fetched_board_columns.len();
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Fetched {board_columns_size} board columns"),
                        Instant::now(),
                    ));
                self.board_columns = fetched_board_columns;
                if self.current_column_index >= self.board_columns.len() {
                    self.current_column_index = self.board_columns.len().saturating_sub(1);
                }
            }
            AppEvent::Board(BoardEvent::BoardColumnUpdated(updated_board_column)) => {
                debug!("Board Column updated: {updated_board_column:?}");
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Board Column updated: {updated_board_column:?}"),
                        Instant::now(),
                    ));
                // load the board columns to reflect the changes
                self.board_service.list_columns();
            }
            AppEvent::Board(BoardEvent::BoardColumnDeleted(deleted_board_column_id)) => {
                debug!("Board Column deleted: {deleted_board_column_id:?}");
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Board Column deleted: {deleted_board_column_id:?}"),
                        Instant::now(),
                    ));
                // load the board columns to reflect the changes
                self.board_service.list_columns();
            }
            AppEvent::Board(BoardEvent::Error(error)) => {
                self.notification_service
                    .send_notification(NotificationMessage::ErrorMsg(
                        format!("Error: {error}"),
                        Instant::now(),
                    ));
            }
            AppEvent::Config(_) => {}
            AppEvent::Card(_) => {}
        }
    }
}

impl EventHandler<(), ()> for ManageBoardDetailsState {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        match self.page_view {
            PageView::BoardDetails => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL)),
                    String::from("Create New Board Column"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL)),
                    String::from("Edit Selected Board Column"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
                    String::from("Delete Selected Board Column"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
                    String::from("Move On The Right"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
                    String::from("Move On The Left"),
                ));
            }
            PageView::ManageBoardColumn => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back to Board List"),
                ));
                include_delegated_event_controls(
                    &mut event_controls,
                    self.manage_board_column_form_state.get_event_controls(),
                );
            }
            PageView::ConfirmDialog => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                    String::from("Move To The Previous Option"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
                    String::from("Move To The Previous Option"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                    String::from("Confirm selection"),
                ));
            }
        }
        event_controls
    }

    fn handle_event(&mut self, event: Event) -> Result<ControlFlow<(), ()>> {
        match self.page_view {
            PageView::BoardDetails => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                            debug!("Handling the manage board column [create new column]");
                            self.manage_board_column_form_state.clear(); // clear the form
                            self.page_view = PageView::ManageBoardColumn;
                        }
                        (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                            debug!("Handling the manage board column [update existing column]");
                            self.manage_board_column_form_state.clear(); // clear the form
                            // preset the form with existing data from selected board column
                            let board_column =
                                self.board_columns[self.current_column_index].clone();
                            self.manage_board_column_form_state
                                .preset_with_board_column_data(board_column);
                            self.page_view = PageView::ManageBoardColumn;
                        }
                        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                            debug!("Handling deleting of selected column");
                            // we will allow the delete of column just in case there are no card available
                            // open the confirmation dialog asking to confirm the delete action for selected board column
                            self.current_action_to_confirm = ActionToConfirm::DeleteBoardColumn;
                            self.page_view = PageView::ConfirmDialog;
                        }
                        (KeyCode::Right, KeyModifiers::NONE) => {
                            if !self.board_columns.is_empty() {
                                debug!("Moving to the next column, moving right");
                                self.current_column_index =
                                    (self.current_column_index + 1) % self.board_columns.len();
                            }
                        }
                        (KeyCode::Left, KeyModifiers::NONE) => {
                            if !self.board_columns.is_empty() {
                                debug!("Moving to the previous column, moving left");
                                if self.current_column_index == 0 {
                                    self.current_column_index = self.board_columns.len() - 1;
                                } else {
                                    self.current_column_index -= 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            PageView::ManageBoardColumn => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => {
                            self.page_view = PageView::BoardDetails
                        }
                        // delegate any other event to the create board form state
                        _ => {
                            let result = self.manage_board_column_form_state.handle_event(event)?;
                            if let ControlFlow::Break((
                                board_column_id,
                                board_column_name,
                                wip_limit,
                                board_column_position,
                            )) = result
                            {
                                // process the collected data from the form
                                if let Some(board_column_id) = board_column_id {
                                    // in this case the id already exists, so it's update
                                    self.board_service.update_column(BoardColumn {
                                        id: board_column_id,
                                        name: board_column_name,
                                        wip_limit: wip_limit.try_into().unwrap(),
                                        position: board_column_position.try_into().unwrap(),
                                        // TODO: API can be better
                                        created_at: NaiveDateTime::default(), // does not matter here, it's not used
                                    });
                                } else {
                                    // in this case the id does not exist, so it's create
                                    self.board_service.create_column(NewBoardColumn::new(
                                        board_column_name,
                                        wip_limit.try_into().unwrap(),
                                        board_column_position.try_into().unwrap(),
                                    ));
                                }
                                return Ok(ControlFlow::Continue(()));
                            }
                        }
                    }
                }
            }
            PageView::ConfirmDialog => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => {
                            // go back to board
                            self.page_view = PageView::BoardDetails;
                        }
                        (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Left, KeyModifiers::NONE) => {
                            let total_options = self.confirm_dialog_state.0.len();
                            self.confirm_dialog_state.1.previous(total_options);
                        }
                        (KeyCode::Down, KeyModifiers::NONE)
                        | (KeyCode::Right, KeyModifiers::NONE) => {
                            let total_options = self.confirm_dialog_state.0.len();
                            self.confirm_dialog_state.1.next(total_options);
                        }
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            let options = &self.confirm_dialog_state.0;
                            let selected_index = self.confirm_dialog_state.1.selected_index;
                            let selected_option = &options[selected_index];

                            let confirmed = selected_option.data;
                            let action = self.current_action_to_confirm.clone();
                            debug!(
                                "About to proceed with selected action: {action:?}, confirmed: {confirmed}"
                            );
                            if selected_option.data {
                                // proceed with the action, right now the only action is delete of board column
                                if self.current_action_to_confirm
                                    == ActionToConfirm::DeleteBoardColumn
                                {
                                    // trigger the deletion of selected board column, fire-and-forget approach
                                    let board_column_id =
                                        &self.board_columns[self.current_column_index].id;
                                    self.board_service.delete_column_by_id(board_column_id);
                                    self.current_action_to_confirm = ActionToConfirm::NoAction;
                                }
                                self.page_view = PageView::BoardDetails;
                            } else {
                                // go back to board
                                self.page_view = PageView::BoardDetails;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(std::ops::ControlFlow::Continue(()))
    }
}

/// `ManageBoardDetails` represents the statefull widget for a page responsible for:
/// - managing the cards of specific Kanban board and columns of Kanban Board
pub struct ManageBoardDetails {}

impl ManageBoardDetails {
    pub fn new() -> Self {
        Self {}
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
        match state.page_view {
            PageView::BoardDetails => {
                // get currently active board
                let active_board = {
                    let config = state.app_config.lock().unwrap();
                    config.current_board.clone()
                };

                let total_columns_num = state.board_columns.len();
                if total_columns_num == 0 {
                    let board_placeholder = Paragraph::new(format!("No Board Columns Yet")).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(ratatui::style::Color::Gray))
                            .title(format!("Manage Board Details - {active_board}"))
                            .title_style(Style::default().bold()),
                    );
                    Widget::render(board_placeholder, area, buf);
                } else {
                    // prepare some styles
                    let normal_style = Style::default();
                    let highlighted_style =
                        Style::default().fg(ratatui::style::Color::Green).bold();

                    // figure out first the number of columns to display
                    let display_columns_num = {
                        let max_columns_display = 4;
                        if total_columns_num > max_columns_display {
                            max_columns_display
                        } else {
                            total_columns_num
                        }
                    };
                    // prepare the place(s) for rendering columns with equal width
                    let constraints = vec![Constraint::Fill(1); display_columns_num];
                    let column_areas = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(&constraints)
                        .split(area);

                    let window_start =
                        state.current_window_start(total_columns_num, display_columns_num);
                    let visible_columns =
                        &state.board_columns[window_start..window_start + display_columns_num];

                    // render the visible board columns evenly across the available area
                    for (index, column_area) in column_areas.iter().enumerate() {
                        let board_column_index = window_start + index;
                        let border_style = if board_column_index == state.current_column_index {
                            highlighted_style
                        } else {
                            normal_style
                        };
                        let board_column = &visible_columns[index];

                        let board_column_name = board_column.name.clone();
                        let wip_limit = board_column.wip_limit;
                        let board_column_position = board_column.position;
                        let column_block = Paragraph::new(board_column.name.clone()).block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("{board_column_name} - (wip: {wip_limit}, position: {board_column_position})"))
                                .title_style(Style::default().bold())
                                .border_style(border_style),
                        );
                        Widget::render(column_block, *column_area, buf);
                    }
                }
            }
            PageView::ManageBoardColumn => {
                // render the manage board column form
                let manage_board_column_form = ManageBoardColumnForm::new();
                let manage_board_column_form_state = &mut state.manage_board_column_form_state;
                StatefulWidget::render(
                    manage_board_column_form,
                    area,
                    buf,
                    manage_board_column_form_state,
                );
            }
            PageView::ConfirmDialog => {
                if state.current_action_to_confirm == ActionToConfirm::DeleteBoardColumn {
                    let selected_board = &state.board_columns[state.current_column_index];
                    let selected_board_name = &selected_board.name;

                    let options = &state.confirm_dialog_state.0;
                    let confirm_dialog = ChoicePicker::new(options)
                        .layout(choice_picker::PickerLayout::VerticalList)
                        .block(
                            Block::default()
                                .title(format!("Do you want to delete selected board column -> {selected_board_name} ?"))
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Red)),
                        );
                    let confirm_dialog_state = &mut state.confirm_dialog_state.1;
                    StatefulWidget::render(
                        confirm_dialog,
                        Center::builder(area)
                            .horizontally(true)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                        confirm_dialog_state,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{AppConfig, KanbanBoard};
    use crate::model::board::{BoardColumn, BoardColumnRepo, BoardColumnRepoError, NewBoardColumn};
    use crate::model::connection::Connection;
    use crate::model::test_utils::{acquire_test_lock, reset_db_pool};
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};
    use crate::tui::components::notification_panel::NotificationMessage;
    use anyhow::Result;
    use chrono::{TimeZone, Utc};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{buffer::Buffer, prelude::Rect};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    fn make_test_app_config(db_path: &Path) -> Arc<Mutex<AppConfig>> {
        let mut boards = HashMap::new();
        boards.insert(
            String::from("test-board"),
            KanbanBoard::new(
                String::from("Test Board"),
                String::from("A temporary board for tests"),
                PathBuf::from(format!("sqlite://{}", db_path.display())),
            ),
        );

        Arc::new(Mutex::new(AppConfig {
            current_board: String::from("test-board"),
            logging_config: Default::default(),
            kanban_boards: boards,
        }))
    }

    async fn init_repo(db_path: &Path) -> Result<BoardColumnRepo> {
        let _lock = acquire_test_lock();
        reset_db_pool();

        let mut boards = HashMap::new();
        boards.insert(
            String::from("test-board"),
            KanbanBoard::new(
                String::from("Test Board"),
                String::from("A temporary board for tests"),
                PathBuf::from(format!("sqlite://{}", db_path.display())),
            ),
        );

        let config = AppConfig {
            current_board: String::from("test-board"),
            logging_config: Default::default(),
            kanban_boards: boards,
        };

        let connection = Connection::new(Arc::new(Mutex::new(config)));
        let pool = connection.get_db_connection_pool().await?;
        Ok(BoardColumnRepo::new(pool))
    }

    async fn make_board_service(
        db_path: &Path,
    ) -> Result<(Arc<BoardService>, mpsc::Receiver<AppEvent>)> {
        let repo = init_repo(db_path).await?;
        let (sender, receiver) = mpsc::channel(10);
        let service = Arc::new(BoardService::new(Arc::new(repo), sender));
        Ok((service, receiver))
    }

    fn make_notification_service() -> (
        Arc<NotificationService>,
        mpsc::Receiver<NotificationMessage>,
    ) {
        let (sender, receiver) = mpsc::channel(10);
        (Arc::new(NotificationService::new(sender)), receiver)
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
    fn test_manage_board_column_form_state_new_initializes_defaults() {
        let state = ManageBoardColumnFormState::new();

        assert!(state.id.is_none());
        assert!(state.name.is_focused());
        assert_eq!(state.wip_limit.get_input_value(), 1);
        assert_eq!(state.position.get_input_value(), 0);
        assert!(!state.wip_limit.is_focused());
        assert!(!state.position.is_focused());
    }

    #[test]
    fn test_manage_board_column_form_state_preset_with_existing_data() {
        let mut state = ManageBoardColumnFormState::new();
        let board_column = BoardColumn {
            id: String::from("col-1"),
            name: String::from("In Progress"),
            wip_limit: 5,
            position: 2,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        };

        state.preset_with_board_column_data(board_column.clone());

        assert_eq!(state.id.as_deref(), Some("col-1"));
        assert_eq!(state.name.get_buffer(), board_column.name);
        assert_eq!(
            state.wip_limit.get_input_value(),
            board_column.wip_limit as i32
        );
        assert_eq!(
            state.position.get_input_value(),
            board_column.position as i32
        );
        assert!(state.name.is_focused());
        assert!(!state.wip_limit.is_focused());
        assert!(!state.position.is_focused());
    }

    #[test]
    fn test_manage_board_column_form_state_clear_resets_values_and_focus() {
        let mut state = ManageBoardColumnFormState::new();
        fill_text_input(&mut state.name, "Test Column");
        state.wip_limit.set_input_value(10).unwrap();
        state.position.set_input_value(3).unwrap();
        state.id = Some(String::from("existing-id"));
        state.current_field = ManageBoardColumnFormField::Position;

        state.clear();

        assert!(state.id.is_none());
        assert_eq!(state.name.get_buffer(), "");
        assert_eq!(state.wip_limit.get_input_value(), 1);
        assert_eq!(state.position.get_input_value(), 0);
        assert!(state.name.is_focused());
        assert!(!state.wip_limit.is_focused());
        assert!(!state.position.is_focused());
        assert!(matches!(
            state.current_field,
            ManageBoardColumnFormField::Name
        ));
    }

    #[test]
    fn test_manage_board_column_form_state_tab_moves_focus_through_fields() {
        let mut state = ManageBoardColumnFormState::new();

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(!state.name.is_focused());
        assert!(state.wip_limit.is_focused());

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(!state.wip_limit.is_focused());
        assert!(state.position.is_focused());

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(!state.position.is_focused());
        assert!(state.name.is_focused());
    }

    #[test]
    fn test_manage_board_column_form_state_submits_when_name_is_populated() {
        let mut state = ManageBoardColumnFormState::new();
        fill_text_input(&mut state.name, "In Progress");
        state.wip_limit.set_input_value(4).unwrap();
        state.position.set_input_value(1).unwrap();

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        match result {
            ControlFlow::Break((id, name, wip_limit, position)) => {
                assert!(id.is_none());
                assert_eq!(name, "In Progress");
                assert_eq!(wip_limit, 4);
                assert_eq!(position, 1);
            }
            _ => panic!("expected submission break"),
        }
    }

    #[test]
    fn test_manage_board_column_form_state_does_not_submit_when_name_is_empty() {
        let mut state = ManageBoardColumnFormState::new();
        state.wip_limit.set_input_value(4).unwrap();
        state.position.set_input_value(1).unwrap();

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(result, ControlFlow::Continue(())));
    }

    #[tokio::test]
    async fn test_manage_board_details_state_new_initializes_board_details_page() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_new.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);

        let state = ManageBoardDetailsState::new(
            app_config.clone(),
            board_service.clone(),
            notification_service.clone(),
        );

        let event = event_receiver
            .recv()
            .await
            .expect("expected board columns fetched event");
        assert!(matches!(
            event,
            AppEvent::Board(BoardEvent::BoardColumnsFetched(_))
        ));
        assert!(matches!(state.page_view, PageView::BoardDetails));
        assert!(state.board_columns.is_empty());
        assert_eq!(state.current_column_index, 0);
        assert!(matches!(
            state.current_action_to_confirm,
            ActionToConfirm::NoAction
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_updates_board_columns_and_clamps_index()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            current_column_index: 10,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let fetched_columns = vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Todo"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }];

        state.handle_app_event(AppEvent::Board(BoardEvent::BoardColumnsFetched(
            fetched_columns.clone(),
        )));

        assert_eq!(state.board_columns, fetched_columns);
        assert_eq!(state.current_column_index, 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_error_sends_notification()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_error.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (notification_service, mut notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state =
            ManageBoardDetailsState::new(app_config, board_service, notification_service);

        let _ = event_receiver.recv().await;

        state.handle_app_event(AppEvent::Board(BoardEvent::Error(
            BoardColumnRepoError::NotFound {
                id: String::from("boom"),
            },
        )));

        let notification = notification_receiver
            .recv()
            .await
            .expect("expected notification message");

        match notification {
            NotificationMessage::ErrorMsg(text, _) => {
                assert!(text.contains("boom"));
            }
            other => panic!("expected error notification, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_n_enters_manage_board_column()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            manage_board_column_form_state: {
                let mut state = ManageBoardColumnFormState::new();
                fill_text_input(&mut state.name, "test");
                state
            },
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ManageBoardColumn));
        assert!(
            state
                .manage_board_column_form_state
                .name
                .get_buffer()
                .is_empty()
        );
        assert!(state.manage_board_column_form_state.name.is_focused());
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_e_presets_existing_column()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumn {
                id: String::from("column-1"),
                name: String::from("In Progress"),
                wip_limit: 2,
                position: 1,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
            }],
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ManageBoardColumn));
        assert_eq!(
            state.manage_board_column_form_state.id.as_deref(),
            Some("column-1")
        );
        assert_eq!(
            state.manage_board_column_form_state.name.get_buffer(),
            "In Progress"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_d_enters_confirm_dialog()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumn {
                id: String::from("column-1"),
                name: String::from("In Progress"),
                wip_limit: 2,
                position: 1,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
            }],
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ConfirmDialog));
        assert!(matches!(
            state.current_action_to_confirm,
            ActionToConfirm::DeleteBoardColumn
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_right_left_cycles_columns() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 1,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                BoardColumn {
                    id: String::from("column-2"),
                    name: String::from("Doing"),
                    wip_limit: 2,
                    position: 1,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
            ],
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Right,
                KeyModifiers::NONE,
            )))
            .unwrap();
        assert_eq!(state.current_column_index, 1);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Right,
                KeyModifiers::NONE,
            )))
            .unwrap();
        assert_eq!(state.current_column_index, 0);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_column_index, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_confirm_dialog_yes_deletes_selected_column()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_delete.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state =
            ManageBoardDetailsState::new(app_config, board_service.clone(), notification_service);
        let _ = event_receiver.recv().await;

        let create_handle =
            board_service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let created_event = event_receiver
            .recv()
            .await
            .expect("expected board column created event");
        create_handle.await.expect("BoardService task panicked");

        let created_column = match created_event {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => column,
            other => panic!("expected created event, got {other:?}"),
        };

        state.page_view = PageView::ConfirmDialog;
        state.current_action_to_confirm = ActionToConfirm::DeleteBoardColumn;
        state.board_columns = vec![created_column.clone()];
        state.current_column_index = 0;

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
            .unwrap();
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::BoardDetails));
        let event = event_receiver
            .recv()
            .await
            .expect("expected board column deleted event");
        assert!(matches!(
            event,
            AppEvent::Board(BoardEvent::BoardColumnDeleted(_))
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_board_details_placeholder_when_no_columns()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_render_empty.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state =
            ManageBoardDetailsState::new(app_config, board_service, notification_service);
        let _ = event_receiver.recv().await;

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf, &mut state);

        let rendered = buffer_to_string(&buf);
        let expected_output = "
┌Manage Board Details - test-board─────┐
│No Board Columns Yet                  │
│                                      │
│                                      │
└──────────────────────────────────────┘";
        assert_rendered_output(&rendered, expected_output);

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_manage_board_column_form_shows_form_fields()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            current_column_index: 0,
            page_view: PageView::ManageBoardColumn,
            current_action_to_confirm: ActionToConfirm::NoAction,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 50, 10);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
Create new column                                 
                                                  
┌Board Column Name:──────────────────────────────┐
│|                                               │
└────────────────────────────────────────────────┘
┌WorkInProgress Limit:───────────────────────────┐
└────────────────────────────────────────────────┘
┌Board Column Position:──────────────────────────┐
│ ▲ 0 ▼                                          │
└────────────────────────────────────────────────┘";

        assert_rendered_output(&rendered, expected_output);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_confirm_dialog_shows_title_and_options() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumn {
                id: String::from("column-1"),
                name: String::from("Todo"),
                wip_limit: 3,
                position: 0,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
            }],
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteBoardColumn,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 80, 6);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
          ┌Do you want to delete selected board column -> Todo ?─────┐          
          │  YES                                                     │          
          │> NO                                                      │          
          │                                                          │          
          │                                                          │          
          └──────────────────────────────────────────────────────────┘          ";

        assert_rendered_output(&rendered, expected_output);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_confirm_dialog_title_includes_column_name()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumn {
                id: String::from("column-1"),
                name: String::from("Todo"),
                wip_limit: 3,
                position: 0,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
            }],
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteBoardColumn,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 90, 6);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
               ┌Do you want to delete selected board column -> Todo ?─────┐               
               │  YES                                                     │               
               │> NO                                                      │               
               │                                                          │               
               │                                                          │               
               └──────────────────────────────────────────────────────────┘               ";

        assert_rendered_output(&rendered, expected_output);

        assert!(rendered.contains("Do you want to delete selected board column -> Todo ?",));
        Ok(())
    }
}
