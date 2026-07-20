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
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};
use textwrap::wrap;
use tracing::debug;

use crate::{
    core::config::AppConfig,
    model::{
        board::{BoardColumn, NewBoardColumn},
        card::{Card, NewCard},
    },
    tui::{
        components::{
            choice_picker::{self, Choice, ChoicePicker, ChoicePickerState},
            integer_input::{IntegerInput, IntegerRange},
            notification_panel::NotificationMessage,
            stateful_window::StatefulWindow,
            text_area::{TextArea, TextAreaState},
            text_input::TextInput,
        },
        event::events::{
            AppEvent::{self},
            BoardEvent, CardEvent,
        },
        handler::include_delegated_event_controls,
        page::common::{Center, EventHandler},
        service::{board::BoardService, card::CardService, notifications::NotificationService},
    },
};

/// `CardStatus` represents the possible values for card's status
#[derive(Debug, Clone, PartialEq, Eq)]
enum CardStatus {
    Active,
    Blocked,
}

/// `ManageCardFormField` represents the enum of form fields available for widget `ManageCardForm`
#[derive(Debug, Clone, PartialEq, Eq)]
enum ManageCardFormField {
    ColumnId,
    Title,
    Description,
    Status,
    BlockedReason,
}

type BoardColumnId = String;

/// `ManageCardFormState` represents the state for statefull widget `ManageCardForm`
#[derive(Debug, Clone)]
struct ManageCardFormState {
    id: Option<String>, // state that will hold the optional id of the card for already existing data
    // form for update vs. form for create
    column_id: (Vec<Choice<BoardColumnId>>, ChoicePickerState), // widget for capturing the association of Kanban Card with Kanban Board Column
    title: TextInput,           // widget for capturing the title of the Kanban Card
    description: TextAreaState, // widget for capturing the main body (markup) of Kanban Card
    status: (Vec<Choice<CardStatus>>, ChoicePickerState), // widget for capturing the status of Kanban Card
    blocked_reason: TextInput, // widget for capturing the reason of Kanban Card being blocked
    current_field: ManageCardFormField,
    // state
    board_columns: Vec<BoardColumn>,
}

impl ManageCardFormState {
    pub fn new(board_columns: Vec<BoardColumn>) -> Self {
        // column id
        let column_id_choices: Vec<Choice<BoardColumnId>> = board_columns
            .iter()
            .map(|board_column| {
                let label = board_column.name.to_string();
                let data = board_column.id.to_string(); // as a data it's enough to keep just the board column id
                Choice::new(&label, data)
            })
            .collect();
        let column_id_choice_picker_state = ChoicePickerState::new(0);

        // title
        let mut title_input = TextInput::new(String::from("")).label(String::from("Card Title:"));
        title_input.focused(false);

        // description
        let mut description_input = TextAreaState::new(String::from("Card Description:"));
        description_input.set_focused(false);

        // status
        let status_choices = vec![
            Choice::new("Active", CardStatus::Active),
            Choice::new("Blocked", CardStatus::Blocked),
        ];
        let status_choice_picker_state = ChoicePickerState::new(0);

        // blocked reason
        let mut blocked_reason_input =
            TextInput::new(String::from("")).label(String::from("Blocked Reason:"));
        blocked_reason_input.focused(false);

        Self {
            id: None,
            column_id: (column_id_choices, column_id_choice_picker_state),
            title: title_input,
            description: description_input,
            status: (status_choices, status_choice_picker_state),
            blocked_reason: blocked_reason_input,
            current_field: ManageCardFormField::ColumnId,
            board_columns,
        }
    }

    pub fn preset_with_card_data(&mut self, card: Card) {
        let Card {
            id,
            column_id,
            title,
            description,
            status,
            blocked_reason,
            created_at: _,
            started_at: _,
            completed_at: _,
        } = card;

        // id
        self.id = Some(id);

        // column id
        let column_id_choices = &self.column_id.0;
        let column_id_choice_picker_state = &mut self.column_id.1;
        column_id_choice_picker_state.selected_index = column_id_choices
            .iter()
            .enumerate()
            .find(|(_index, choice)| choice.data == column_id)
            .unwrap()
            .0;

        // title
        self.title = TextInput::new(title).label(String::from("Card Title:"));
        self.title.focused(false);

        // description
        self.description = TextAreaState::new(String::from("Card Description:"));
        self.description.set_focused(false);
        self.description.set_text(
            description
                .map(|text| {
                    text.split("\n")
                        .into_iter()
                        .map(|line| line.to_string())
                        .collect()
                })
                .unwrap_or_default(),
        );

        // status
        let status_choices = &self.status.0;
        let status_choice_picker_state = &mut self.status.1;
        status_choice_picker_state.selected_index = status_choices
            .iter()
            .enumerate()
            .find(|(index, choice)| choice.label == status)
            .unwrap()
            .0;

        // blocked reason
        self.blocked_reason = TextInput::new(blocked_reason.unwrap_or_default())
            .label(String::from("Blocked Reason:"));
        self.blocked_reason.focused(false);

        self.current_field = ManageCardFormField::ColumnId;
    }

    pub fn set_board_columns(&mut self, board_columns: Vec<BoardColumn>) {
        // self.board_columns = board_columns;
        let column_id_choices: Vec<Choice<BoardColumnId>> = board_columns
            .iter()
            .map(|board_column| {
                let label = board_column.name.to_string();
                let data = board_column.id.to_string(); // as a data it's enough to keep just the board column id
                Choice::new(&label, data)
            })
            .collect();
        let column_id_choice_picker_state = ChoicePickerState::new(0);
        self.column_id = (column_id_choices, column_id_choice_picker_state);
    }

    pub fn clear(&mut self) {
        // id
        self.id = None;

        // column id
        let column_id_choice_picker_state = &mut self.column_id.1;
        column_id_choice_picker_state.selected_index = 0;

        // title
        self.title.clear();
        self.title.focused(false);

        // description
        self.description = TextAreaState::new(String::from("Card Description:"));
        self.description.set_focused(false);

        // status
        let status_choice_picker_state = &mut self.status.1;
        status_choice_picker_state.selected_index = 0;

        // blocked reason
        self.blocked_reason.clear();
        self.blocked_reason.focused(false);

        self.current_field = ManageCardFormField::ColumnId;
    }
}

impl
    EventHandler<
        (
            Option<String>,
            BoardColumnId,
            String,
            String,
            CardStatus,
            String,
        ),
        (),
    > for ManageCardFormState
{
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            String::from("Move To Next Field"),
        ));
        match self.current_field {
            ManageCardFormField::ColumnId => {
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
            ManageCardFormField::Title => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.title.get_event_controls(),
                );
            }
            ManageCardFormField::Description => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.description.get_event_controls(),
                );
            }
            ManageCardFormField::Status => {
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
            ManageCardFormField::BlockedReason => {
                include_delegated_event_controls(
                    &mut event_controls,
                    self.blocked_reason.get_event_controls(),
                );
            }
        }
        event_controls
    }
    fn handle_event(
        &mut self,
        event: Event,
    ) -> Result<
        ControlFlow<
            (
                Option<String>,
                BoardColumnId,
                String,
                String,
                CardStatus,
                String,
            ),
            (),
        >,
    > {
        if let Event::Key(key_event) = event
            && (key_event.code == KeyCode::Enter && key_event.modifiers == KeyModifiers::NONE)
            && (self.title.get_buffer() != "")
            && (self.current_field != ManageCardFormField::Description)
        // The Enter key is valid for Description/TextArea
        {
            // id
            let id = self.id.clone();
            // column id
            let column_id_choices = &self.column_id.0;
            let column_id = column_id_choices[self.column_id.1.selected_index]
                .data
                .clone();
            // title
            let title = self.title.get_buffer();
            // description
            let description = self.description.get_text_as_string();
            // status
            let status_choices = &self.status.0;
            let status = status_choices[self.status.1.selected_index].data.clone();
            // blocked reason
            let blocked_reason = self.blocked_reason.get_buffer();
            // self.clear();
            return Ok(ControlFlow::Break((
                id,
                column_id,
                title,
                description,
                status,
                blocked_reason,
            )));
        } else {
            match self.current_field {
                ManageCardFormField::ColumnId => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.title.focused(true);
                                self.current_field = ManageCardFormField::Title;
                            }
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                let total_options = self.column_id.0.len();
                                self.column_id.1.previous(total_options);
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                let total_options = self.column_id.0.len();
                                self.column_id.1.next(total_options);
                            }
                            _ => {}
                        }
                    }
                }
                ManageCardFormField::Title => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.title.focused(false);
                                self.description.set_focused(true);
                                self.current_field = ManageCardFormField::Description;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let title_input_text = &mut self.title;
                                let _result = title_input_text.handle_event(event)?;
                            }
                        }
                    }
                }
                ManageCardFormField::Description => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.description.set_focused(false);
                                self.current_field = ManageCardFormField::Status;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let description_input_text = &mut self.description;
                                let _result = description_input_text.handle_event(event)?;
                            }
                        }
                    }
                }
                ManageCardFormField::Status => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.blocked_reason.focused(true);
                                self.current_field = ManageCardFormField::BlockedReason;
                            }
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                let total_options = self.status.0.len();
                                self.status.1.previous(total_options);
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                let total_options = self.status.0.len();
                                self.status.1.next(total_options);
                            }
                            _ => {}
                        }
                    }
                }
                ManageCardFormField::BlockedReason => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                self.blocked_reason.focused(false);
                                self.current_field = ManageCardFormField::ColumnId;
                            }
                            // any other event delegate to the underlying widget
                            _ => {
                                let blocked_reason_input_text = &mut self.blocked_reason;
                                let _result = blocked_reason_input_text.handle_event(event)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(ControlFlow::Continue(()))
    }
}

/// `ManageCardForm` represents the statefull widget
#[derive(Debug)]
struct ManageCardForm {}

impl ManageCardForm {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for ManageCardForm {
    type State = ManageCardFormState;
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
            Constraint::Length(15),
            Constraint::Length(3),
            Constraint::Length(3),
        ]);
        let [
            form_header_area,
            column_id_field_area,
            title_area,
            description_area,
            status_area,
            blocked_reason_area,
        ] = area.layout(&vertical_layout);

        // render the form header above the form widgets
        let form_header = if let Some(id) = &state.id {
            Text::from(format!("Update of card (id: {id})"))
                .bold()
                .fg(Color::Green)
        } else {
            Text::from("Create new card").bold().fg(Color::Green)
        };
        Widget::render(
            form_header,
            Center::builder(form_header_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );

        // render the column id choice picker
        let column_id_border_style = if state.current_field == ManageCardFormField::ColumnId {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::Gray)
        };
        let (column_id_options, column_id_choice_picker_state) = &mut state.column_id;
        let column_id_choice_picker = ChoicePicker::new(column_id_options)
            .layout(choice_picker::PickerLayout::HorizontalCarousel)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(column_id_border_style)
                    .title("Column Id:"),
            );
        StatefulWidget::render(
            column_id_choice_picker,
            Center::builder(column_id_field_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
            column_id_choice_picker_state,
        );

        // render the title
        let title_input = &state.title;
        Widget::render(
            title_input,
            Center::builder(title_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );

        // render the description
        let description_input_state = &mut state.description;
        let description_text_area = TextArea::new();
        StatefulWidget::render(
            description_text_area,
            Center::builder(description_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
            description_input_state,
        );

        // render the status choice picker
        let status_border_style = if state.current_field == ManageCardFormField::Status {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::Gray)
        };
        let (status_options, status_choice_picker_state) = &mut state.status;
        let status_choice_picker = ChoicePicker::new(status_options)
            .layout(choice_picker::PickerLayout::HorizontalCarousel)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(status_border_style)
                    .title("Status:"),
            );
        StatefulWidget::render(
            status_choice_picker,
            Center::builder(status_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
            status_choice_picker_state,
        );

        // render the blocked reason
        let blocked_reason_input = &state.blocked_reason;
        Widget::render(
            blocked_reason_input,
            Center::builder(blocked_reason_area)
                .horizontally(true)
                .vertically(false)
                .build()
                .center(),
            buf,
        );
    }
}

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
            Text::from("Create new column").bold().fg(Color::Green)
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
    ManageCard,
    ViewCard,
    ConfirmDialog,
}

/// `ActionToConfirm` represents all the action on thos page that needs to be confirmed before proceeding further
#[derive(Debug, Clone, PartialEq)]
enum ActionToConfirm {
    NoAction,
    DeleteBoardColumn,
    DeleteCard,
}

#[derive(Debug, Clone)]
struct BoardColumnWithCards {
    pub board_column: BoardColumn,
    pub cards: Vec<Card>,
    pub list_state: ListState,
}

impl BoardColumnWithCards {
    pub fn new(board_column: BoardColumn, cards: Vec<Card>) -> Self {
        Self {
            board_column,
            cards,
            list_state: ListState::default(),
        }
    }
}

/// `ManageBoardDetailsState` represents the state for statefull widget `ManageBoardDetails`
#[derive(Clone)]
pub struct ManageBoardDetailsState {
    app_config: Arc<Mutex<AppConfig>>,
    board_service: Arc<BoardService>,
    card_service: Arc<CardService>,
    notification_service: Arc<NotificationService>,
    // page state
    board_columns: Vec<BoardColumnWithCards>,
    card_to_view: Option<Card>,
    current_column_index: usize,
    page_view: PageView,
    current_action_to_confirm: ActionToConfirm,
    view_card_scroll_offset: u16,
    manage_board_column_form_state: ManageBoardColumnFormState,
    manage_card_form_state: ManageCardFormState,
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
        card_service: Arc<CardService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        // trigger the board columns fetching, fire-and-forget call
        board_service.list_columns();
        Self {
            app_config,
            board_service,
            card_service,
            notification_service,
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
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
                self.board_columns = fetched_board_columns
                    // .clone()
                    .iter()
                    .map(|board_column| BoardColumnWithCards::new(board_column.clone(), vec![]))
                    .collect();
                self.manage_card_form_state
                    .set_board_columns(fetched_board_columns);
                if self.current_column_index >= self.board_columns.len() {
                    self.current_column_index = self.board_columns.len().saturating_sub(1);
                }
                // trigger the card loading/fetching
                self.board_columns
                    .iter()
                    .for_each(|board_column_with_cards| {
                        let column_id = &board_column_with_cards.board_column.id;
                        self.card_service.list_columns(column_id);
                    });
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
            AppEvent::Card(CardEvent::CardCreated(created_card)) => {
                debug!("New Card created: {created_card:?}");
                let card_title = &created_card.title;
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("New Card created, title: {card_title}"),
                        Instant::now(),
                    ));
                let column_id = created_card.column_id.clone();
                for board_column_with_cards in &mut self.board_columns {
                    if board_column_with_cards.board_column.id == column_id {
                        board_column_with_cards.cards.push(created_card.clone());
                    }
                }
            }
            AppEvent::Card(CardEvent::CardFetched(fetched_card)) => {
                debug!("Card fetched");
                let column_id = fetched_card.column_id.clone();
                for board_column_with_cards in &mut self.board_columns {
                    if board_column_with_cards.board_column.id == column_id {
                        // TODO: maybe in cause of existing card we should replace the card in the column rathe than push
                        // in order to eliminate duplicates
                        board_column_with_cards.cards.push(fetched_card.clone());
                    }
                }
            }
            AppEvent::Card(CardEvent::CardsFetched(fetched_cards)) => {
                debug!("Cards fetched");
                for card in fetched_cards {
                    let column_id = card.column_id.clone();
                    for board_column_with_cards in &mut self.board_columns {
                        if board_column_with_cards.board_column.id == column_id {
                            board_column_with_cards.cards.push(card.clone());
                        }
                    }
                }
            }
            AppEvent::Card(CardEvent::CardUpdated(updated_card)) => {
                let card_id = &updated_card.id;
                debug!("Card updated: {card_id}");
                for board_column_with_cards in &mut self.board_columns {
                    board_column_with_cards
                        .cards
                        .retain(|card| card.id != *card_id);
                    if board_column_with_cards.board_column.id == updated_card.column_id {
                        board_column_with_cards.cards.push(updated_card.clone());
                    }
                }
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Card updated, id: {card_id}"),
                        Instant::now(),
                    ));
            }
            AppEvent::Card(CardEvent::CardDeleted(deleted_card)) => {
                debug!("Card deleted: {deleted_card}");
                for board_column_with_cards in &mut self.board_columns {
                    board_column_with_cards
                        .cards
                        .retain(|card| card.id != deleted_card);
                }
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Card deleted, id: {deleted_card}"),
                        Instant::now(),
                    ));
            }
            AppEvent::Card(CardEvent::Error(error)) => {
                self.notification_service
                    .send_notification(NotificationMessage::ErrorMsg(
                        format!("Error: {error}"),
                        Instant::now(),
                    ));
            }
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
                    Event::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL)),
                    String::from("Create New Card"),
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
                    Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
                    String::from("Remove Selected Card"),
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
            PageView::ManageCard => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back to Board List"),
                ));
                include_delegated_event_controls(
                    &mut event_controls,
                    self.manage_card_form_state.get_event_controls(),
                );
            }
            PageView::ViewCard => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back to Board List"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                    String::from("Scroll Card Up"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
                    String::from("Scroll Card Down"),
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
                        (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                            debug!("Handling the manage card [create new card]");
                            self.manage_card_form_state.clear(); // clear the form
                            self.page_view = PageView::ManageCard;
                        }
                        (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                            debug!("Handling the manage board column [update existing column]");
                            self.manage_board_column_form_state.clear(); // clear the form
                            // preset the form with existing data from selected board column
                            let board_column = self.board_columns[self.current_column_index]
                                .board_column
                                .clone();
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
                        (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                            debug!(
                                "Handling removing of selected card from currently highlighted column"
                            );
                            let card_list_state =
                                self.board_columns[self.current_column_index].list_state;
                            if card_list_state.selected().is_some() {
                                self.current_action_to_confirm = ActionToConfirm::DeleteCard;
                                self.page_view = PageView::ConfirmDialog;
                            }
                        }
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            debug!(
                                "Handling detail view of selected card from currently highlighted column"
                            );
                            let card_list_state =
                                self.board_columns[self.current_column_index].list_state;
                            if let Some(selected_card_index) = card_list_state.selected() {
                                let cards = &self.board_columns[self.current_column_index].cards;
                                self.card_to_view = Some(cards[selected_card_index].clone());
                                self.view_card_scroll_offset = 0;
                                self.page_view = PageView::ViewCard;
                            }
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
                        (KeyCode::Up, KeyModifiers::NONE) => {
                            if !self.board_columns.is_empty() {
                                debug!("Moving up the cards in a column");
                                let selected_board_column =
                                    &mut self.board_columns[self.current_column_index];
                                let card_list_state = &mut selected_board_column.list_state;
                                card_list_state.select_previous();
                            }
                        }
                        (KeyCode::Down, KeyModifiers::NONE) => {
                            if !self.board_columns.is_empty() {
                                debug!("Moving down the cards in a column");
                                let selected_board_column =
                                    &mut self.board_columns[self.current_column_index];
                                let card_list_state = &mut selected_board_column.list_state;
                                card_list_state.select_next();
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
                                    let board_column_id = &self.board_columns
                                        [self.current_column_index]
                                        .board_column
                                        .id;
                                    self.board_service.delete_column_by_id(board_column_id);
                                    self.current_action_to_confirm = ActionToConfirm::NoAction;
                                } else if self.current_action_to_confirm
                                    == ActionToConfirm::DeleteCard
                                {
                                    // trigger the removal of selected card, fire-and-forget approach
                                    let column_cards =
                                        &self.board_columns[self.current_column_index].cards;
                                    let card_list_state =
                                        &self.board_columns[self.current_column_index].list_state;
                                    if let Some(selected_card_index) = card_list_state.selected() {
                                        let card_to_delete = &column_cards[selected_card_index];
                                        self.card_service.delete_card_by_id(&card_to_delete.id);
                                    }
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
            PageView::ManageCard => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => {
                            // go back to board
                            self.page_view = PageView::BoardDetails;
                        }
                        _ => {
                            // delegate the event to the underlying state
                            let result = self.manage_card_form_state.handle_event(event)?;
                            // TODO: based on the result decide if to create or update a card
                            if let ControlFlow::Break((
                                card_id,
                                column_id,
                                title,
                                description,
                                status,
                                blocked_reason,
                            )) = result
                            {
                                // process the collected data from the form
                                if let Some(card_id) = card_id {
                                    // in this case the id already exists, so it's update
                                    let status = if status == CardStatus::Active {
                                        "Active"
                                    } else {
                                        "Blocked"
                                    };
                                    self.card_service.update_card(Card {
                                        id: card_id,
                                        column_id,
                                        title,
                                        description: Some(description),
                                        status: status.to_string(),
                                        blocked_reason: Some(blocked_reason),
                                        // TODO: API can be better
                                        created_at: NaiveDateTime::default(),
                                        started_at: None,
                                        completed_at: None,
                                    });
                                } else {
                                    // in this case the id does not exist, so it's create
                                    let status = if status == CardStatus::Active {
                                        "Active"
                                    } else {
                                        "Blocked"
                                    };
                                    self.card_service.create_card(NewCard::new(
                                        column_id,
                                        title,
                                        Some(description),
                                        status.to_string(),
                                        Some(blocked_reason),
                                    ));
                                }
                                return Ok(ControlFlow::Continue(()));
                            }
                        }
                    }
                }
            }
            PageView::ViewCard => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => {
                            self.page_view = PageView::BoardDetails
                        }
                        (KeyCode::Up, KeyModifiers::NONE) => {
                            if self.view_card_scroll_offset > 0 {
                                self.view_card_scroll_offset -= 1;
                            }
                        }
                        (KeyCode::Down, KeyModifiers::NONE) => {
                            self.view_card_scroll_offset =
                                self.view_card_scroll_offset.saturating_add(1);
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
                    let board_placeholder = Paragraph::new("No Board Columns Yet".to_string())
                        .block(
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
                        &mut state.board_columns[window_start..window_start + display_columns_num];

                    // render the visible board columns evenly across the available area
                    for (index, column_area) in column_areas.iter().enumerate() {
                        let board_column_index = window_start + index;
                        let border_style = if board_column_index == state.current_column_index {
                            highlighted_style
                        } else {
                            normal_style
                        };
                        let board_column = &visible_columns[index].board_column;
                        let cards = &visible_columns[index].cards;
                        let cards_list_state = &mut visible_columns[index].list_state;

                        let board_column_name = board_column.name.clone();
                        let wip_limit = board_column.wip_limit;
                        let board_column_position = board_column.position;
                        let card_items: Vec<ListItem> = cards
                            .iter()
                            .map(|card| {
                                ListItem::new(Text::from(Line::from(card.title.to_string())))
                            })
                            .collect();
                        let column_cards_list = List::default().items(card_items).highlight_style(
                            Style::default().fg(Color::LightYellow).bg(Color::DarkGray),
                        );
                        let column_block_window = StatefulWindow::new(
                            &format!(
                                "{board_column_name} - (wip: {wip_limit}, position: {board_column_position})"
                            ),
                            column_cards_list,
                        ).title_style(Style::default().bold()).border_style(border_style);
                        StatefulWidget::render(
                            column_block_window,
                            *column_area,
                            buf,
                            cards_list_state,
                        );
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
                    let selected_board =
                        &state.board_columns[state.current_column_index].board_column;
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
                } else if state.current_action_to_confirm == ActionToConfirm::DeleteCard {
                    let card_list_state =
                        &state.board_columns[state.current_column_index].list_state;
                    if let Some(selected_index) = card_list_state.selected() {
                        let selected_card_title = &state.board_columns[state.current_column_index]
                            .cards[selected_index]
                            .title;

                        let options = &state.confirm_dialog_state.0;
                        let confirm_dialog = ChoicePicker::new(options)
                        .layout(choice_picker::PickerLayout::VerticalList)
                        .block(
                            Block::default()
                                .title(format!(
                                    "Do you want to delete selected card -> {selected_card_title} ?"
                                ))
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
            PageView::ManageCard => {
                // render the manage card form
                let manage_card_form = ManageCardForm::new();
                let manage_card_form_state = &mut state.manage_card_form_state;
                StatefulWidget::render(manage_card_form, area, buf, manage_card_form_state);
            }
            PageView::ViewCard => {
                if let Some(card) = &state.card_to_view {
                    let Card {
                        title,
                        status,
                        blocked_reason,
                        description,
                        created_at,
                        started_at,
                        completed_at,
                        id: _id,
                        column_id: _column_id,
                    } = card;

                    // prepare the layout and the places for rendering
                    let (
                        title_area,
                        status_created_at_area,
                        maybe_blocked_reason_area,
                        description_area,
                    ) = if let Some(blocked_reason) = blocked_reason
                        && blocked_reason.len() != 0
                    {
                        let vertical_layout = Layout::vertical([
                            Constraint::Length(2), // place for title
                            Constraint::Length(2), // place for status + created_at
                            Constraint::Length(2), // place for blocked_reason
                            Constraint::Fill(1),   // place for description
                        ]);
                        let [
                            title_area,
                            status_created_at_area,
                            blocked_reason_area,
                            description_area,
                        ] = area.layout(&vertical_layout);
                        (
                            title_area,
                            status_created_at_area,
                            Some(blocked_reason_area),
                            description_area,
                        )
                    } else {
                        let vertical_layout = Layout::vertical([
                            Constraint::Length(2), // place for title
                            Constraint::Length(2), // place for status + created_at
                            Constraint::Fill(1),   // place for description
                        ]);
                        let [title_area, status_created_at_area, description_area] =
                            area.layout(&vertical_layout);
                        (title_area, status_created_at_area, None, description_area)
                    };

                    let horizontal_row_layout =
                        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]);
                    let [status_area, created_at_area] =
                        status_created_at_area.layout(&horizontal_row_layout);

                    // render title
                    let title_paragraph = Paragraph::new(format!("{title}"));
                    Widget::render(
                        title_paragraph,
                        Center::builder(title_area)
                            .horizontally(false)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );

                    // render the status
                    let status_style = if status == "Active" {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    };
                    let status_paragraph =
                        Paragraph::new(format!("status: {status}")).style(status_style);
                    Widget::render(
                        status_paragraph,
                        Center::builder(status_area)
                            .horizontally(false)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );

                    // render the created_at
                    let created_at_paragraph = Paragraph::new(format!("created: {created_at}"));
                    Widget::render(
                        created_at_paragraph,
                        Center::builder(created_at_area)
                            .horizontally(false)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );

                    // render the optional blocked_reason
                    if let Some(blocked_reason) = blocked_reason
                        && let Some(blocked_reason_area) = maybe_blocked_reason_area
                    {
                        let blocked_reason_paragraph =
                            Paragraph::new(format!("reason: {blocked_reason}"))
                                .style(Style::default().fg(Color::Red));
                        Widget::render(
                            blocked_reason_paragraph,
                            Center::builder(blocked_reason_area)
                                .horizontally(false)
                                .vertically(false)
                                .build()
                                .center(),
                            buf,
                        );
                    }

                    // render the description markup with manual scrolling
                    if let Some(description) = description {
                        let wrapped_lines = wrap(description, description_area.width as usize);
                        let list_items: Vec<ListItem> = wrapped_lines
                            .iter()
                            .map(|line| ListItem::new(Text::from(Line::from(line.as_ref()))))
                            .collect();
                        let mut list_state = ListState::default();
                        if !list_items.is_empty() {
                            let selected_index = state
                                .view_card_scroll_offset
                                .min((list_items.len().saturating_sub(1)) as u16)
                                as usize;
                            list_state.select(Some(selected_index));
                        }
                        let description_list = List::new(list_items);
                        StatefulWidget::render(
                            description_list,
                            Center::builder(description_area)
                                .horizontally(false)
                                .vertically(false)
                                .build()
                                .center(),
                            buf,
                            &mut list_state,
                        );
                    }
                } else {
                    // this should not happen, but if yes, then let user know there is no card to display
                    let paragraph = Paragraph::new("No card to display");
                    Widget::render(
                        paragraph,
                        Center::builder(area)
                            .horizontally(true)
                            .vertically(true)
                            .build()
                            .center(),
                        buf,
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
    use crate::model::card::CardRepo;
    use crate::model::connection::Connection;
    use crate::model::test_utils::{acquire_test_lock, reset_db_pool};
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};
    use crate::tui::components::notification_panel::NotificationMessage;
    use anyhow::Result;
    use chrono::{TimeZone, Utc};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{buffer::Buffer, prelude::Rect};
    use sqlx::encode::IsNull::No;
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

    async fn init_repos(db_path: &Path) -> Result<(BoardColumnRepo, CardRepo)> {
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
        Ok((BoardColumnRepo::new(pool.clone()), CardRepo::new(pool)))
    }

    async fn make_board_service(
        db_path: &Path,
    ) -> Result<(Arc<BoardService>, mpsc::Receiver<AppEvent>)> {
        let repo = init_repos(db_path).await?;
        let (sender, receiver) = mpsc::channel(10);
        let service = Arc::new(BoardService::new(Arc::new(repo.0), sender));
        Ok((service, receiver))
    }

    async fn make_card_service(
        db_path: &Path,
    ) -> Result<(Arc<CardService>, mpsc::Receiver<AppEvent>)> {
        let repo = init_repos(db_path).await?;
        let (sender, receiver) = mpsc::channel(10);
        let service = Arc::new(CardService::new(Arc::new(repo.1), sender));
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
        let (card_service, mut _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);

        let state = ManageBoardDetailsState::new(
            app_config.clone(),
            board_service.clone(),
            card_service.clone(),
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
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 10,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
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

        assert_eq!(
            state
                .board_columns
                .iter()
                .map(|board_column_tuple| board_column_tuple.board_column.clone())
                .collect::<Vec<BoardColumn>>(),
            fetched_columns
        );
        assert_eq!(state.current_column_index, 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_error_sends_notification()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_error.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, mut notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service,
            card_service,
            notification_service,
        );

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
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: {
                let mut state = ManageBoardColumnFormState::new();
                fill_text_input(&mut state.name, "test");
                state
            },
            manage_card_form_state: ManageCardFormState::new(vec![]),
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
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("In Progress"),
                    wip_limit: 2,
                    position: 1,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
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
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("In Progress"),
                    wip_limit: 2,
                    position: 1,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
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
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![
                BoardColumnWithCards::new(
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
                    vec![],
                ),
                BoardColumnWithCards::new(
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
                    vec![],
                ),
            ],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
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
        let (card_service, mut _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service,
            notification_service,
        );
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
        state.board_columns = vec![BoardColumnWithCards::new(created_column.clone(), vec![])];
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
        let (card_service, mut _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service,
            card_service,
            notification_service,
        );
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
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageBoardColumn,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
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
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteBoardColumn,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
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
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteBoardColumn,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
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

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_view_card_scrolls_up_down() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: Some(Card {
                id: String::from("card-1"),
                column_id: String::from("column-1"),
                title: String::from("View Card Title"),
                description: Some(String::from(
                    "Line one line two line three line four line five line six line seven line eight line nine line ten",
                )),
                status: String::from("Active"),
                blocked_reason: None,
                created_at: NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
                started_at: None,
                completed_at: None,
            }),
            current_column_index: 0,
            page_view: PageView::ViewCard,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.view_card_scroll_offset, 1);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.view_card_scroll_offset, 0);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.view_card_scroll_offset, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_view_card_scrolls_description_in_output() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: Some(Card {
                id: String::from("card-1"),
                column_id: String::from("column-1"),
                title: String::from("View Card Title"),
                description: Some(String::from(
                    "Line one Line two Line three Line four Line five",
                )),
                status: String::from("Active"),
                blocked_reason: None,
                created_at: NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
                started_at: None,
                completed_at: None,
            }),
            current_column_index: 0,
            page_view: PageView::ViewCard,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let area = Rect::new(0, 0, 20, 6);
        let mut buf = Buffer::empty(area);
        let widget = ManageBoardDetails::new();

        widget.render(area, &mut buf, &mut state);
        let rendered0 = buffer_to_string(&buf);

        assert!(rendered0.contains("Line one"));
        assert!(rendered0.contains("Line two"));

        state.view_card_scroll_offset = 4;
        let mut buf = Buffer::empty(area);
        let widget = ManageBoardDetails::new();
        widget.render(area, &mut buf, &mut state);
        let rendered1 = buffer_to_string(&buf);

        assert!(rendered1.contains("Line five"));
        assert_ne!(rendered0, rendered1);

        Ok(())
    }

    #[test]
    fn test_manage_card_form_state_new_initializes_defaults() {
        let board_columns = vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }];

        let state = ManageCardFormState::new(board_columns.clone());

        assert!(state.id.is_none());
        assert_eq!(state.column_id.0.len(), 1);
        assert_eq!(state.column_id.0[0].label, "Backlog");
        assert_eq!(state.column_id.0[0].data, "column-1");
        assert_eq!(state.column_id.1.selected_index, 0);
        assert_eq!(state.title.get_buffer(), "");
        assert!(!state.title.is_focused());
        assert_eq!(state.description.get_text_as_string(), "");
        assert!(!state.description.is_focused());
        assert_eq!(state.status.1.selected_index, 0);
        assert_eq!(state.blocked_reason.get_buffer(), "");
        assert!(!state.blocked_reason.is_focused());
        assert!(matches!(state.current_field, ManageCardFormField::ColumnId));
    }

    #[test]
    fn test_manage_card_form_state_preset_with_card_data_sets_fields() {
        let board_columns = vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }];
        let mut state = ManageCardFormState::new(board_columns);
        let card = Card {
            id: String::from("card-1"),
            column_id: String::from("column-1"),
            title: String::from("Fix bug"),
            description: Some(String::from("Line one\nLine two")),
            status: String::from("Blocked"),
            blocked_reason: Some(String::from("Waiting on API")),
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
            started_at: None,
            completed_at: None,
        };

        state.preset_with_card_data(card);

        assert_eq!(state.id.as_deref(), Some("card-1"));
        assert_eq!(state.column_id.1.selected_index, 0);
        assert_eq!(state.title.get_buffer(), "Fix bug");
        assert_eq!(state.description.get_text_as_string(), "Line one\nLine two");
        assert_eq!(state.status.1.selected_index, 1);
        assert_eq!(state.blocked_reason.get_buffer(), "Waiting on API");
        assert!(matches!(state.current_field, ManageCardFormField::ColumnId));
    }

    #[test]
    fn test_manage_card_form_state_set_board_columns_resets_choices() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        state.set_board_columns(vec![BoardColumn {
            id: String::from("column-2"),
            name: String::from("In Progress"),
            wip_limit: 2,
            position: 1,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        assert_eq!(state.column_id.0.len(), 1);
        assert_eq!(state.column_id.0[0].label, "In Progress");
        assert_eq!(state.column_id.0[0].data, "column-2");
        assert_eq!(state.column_id.1.selected_index, 0);
    }

    #[test]
    fn test_manage_card_form_state_clear_resets_state() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);
        state.id = Some(String::from("card-1"));
        fill_text_input(&mut state.title, "Fix bug");
        state.description.set_text(vec![String::from("Line one")]);
        state.status.1.selected_index = 1;
        fill_text_input(&mut state.blocked_reason, "Blocked reason");
        state.current_field = ManageCardFormField::BlockedReason;

        state.clear();

        assert!(state.id.is_none());
        assert_eq!(state.column_id.1.selected_index, 0);
        assert_eq!(state.title.get_buffer(), "");
        assert_eq!(state.description.get_text_as_string(), "");
        assert_eq!(state.status.1.selected_index, 0);
        assert_eq!(state.blocked_reason.get_buffer(), "");
        assert!(matches!(state.current_field, ManageCardFormField::ColumnId));
    }

    #[test]
    fn test_manage_card_form_state_tab_moves_focus_through_fields() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(state.title.is_focused());
        assert!(matches!(state.current_field, ManageCardFormField::Title));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(state.description.is_focused());
        assert!(matches!(
            state.current_field,
            ManageCardFormField::Description
        ));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(matches!(state.current_field, ManageCardFormField::Status));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(matches!(
            state.current_field,
            ManageCardFormField::BlockedReason
        ));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(matches!(state.current_field, ManageCardFormField::ColumnId));
        assert!(!state.blocked_reason.is_focused());
    }

    #[test]
    fn test_manage_card_form_state_enter_in_description_does_not_submit() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);
        fill_text_input(&mut state.title, "Fix bug");

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(matches!(
            state.current_field,
            ManageCardFormField::Description
        ));

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(result, ControlFlow::Continue(())));
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_k_enters_manage_card() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: {
                let mut state = ManageCardFormState::new(vec![BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Backlog"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                }]);
                fill_text_input(&mut state.title, "Should be cleared");
                state
            },
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('k'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ManageCard));
        assert!(state.manage_card_form_state.title.get_buffer().is_empty());
        assert!(matches!(
            state.manage_card_form_state.current_field,
            ManageCardFormField::ColumnId
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_confirm_dialog_yes_deletes_selected_card() -> Result<()>
    {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_delete_card.db");
        let (board_service, mut board_event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut card_event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service.clone(),
            notification_service,
        );
        let _ = board_event_receiver.recv().await;

        let create_column_handle =
            board_service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let created_column = match board_event_receiver
            .recv()
            .await
            .expect("expected board column created event")
        {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => column,
            other => panic!("expected created event, got {other:?}"),
        };
        create_column_handle
            .await
            .expect("BoardService task panicked");

        let create_card_handle = card_service.create_card(NewCard::new(
            created_column.id.clone(),
            String::from("Card 1"),
            Some(String::from("Description")),
            String::from("Active"),
            None,
        ));
        let created_card = match card_event_receiver
            .recv()
            .await
            .expect("expected card created event")
        {
            AppEvent::Card(CardEvent::CardCreated(card)) => card,
            other => panic!("expected created event, got {other:?}"),
        };
        create_card_handle.await.expect("CardService task panicked");

        let mut list_state = ListState::default();
        list_state.select(Some(0));
        state.page_view = PageView::ConfirmDialog;
        state.current_action_to_confirm = ActionToConfirm::DeleteCard;
        state.board_columns = vec![BoardColumnWithCards::new(
            created_column.clone(),
            vec![created_card.clone()],
        )];
        state.board_columns[0].list_state = list_state;
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
        let event = card_event_receiver
            .recv()
            .await
            .expect("expected card deleted event");
        assert!(matches!(event, AppEvent::Card(CardEvent::CardDeleted(_))));

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_manage_card_form_shows_form_title() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageCard,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![BoardColumn {
                id: String::from("column-1"),
                name: String::from("Todo"),
                wip_limit: 3,
                position: 0,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
            }]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 60, 24);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
Create new card                                             
                                                            
┌Column Id:────────────────────────────────────────────────┐
│                         < Todo >                         │
└──────────────────────────────────────────────────────────┘
┌Card Title:───────────────────────────────────────────────┐
│|                                                         │
└──────────────────────────────────────────────────────────┘
┌Card Description:─────────────────────────────────────────┐
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
└──────────────────────────────────────────────────────────┘
┌Status:───────────────────────────────────────────────────┐
│                        < Active >                        │
└──────────────────────────────────────────────────────────┘
┌Blocked Reason:───────────────────────────────────────────┐
│|                                                         │
└──────────────────────────────────────────────────────────┘";

        assert_rendered_output(&rendered, expected_output);

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_manage_card_form_exact_output() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageCard,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![BoardColumn {
                id: String::from("column-1"),
                name: String::from("Todo"),
                wip_limit: 3,
                position: 0,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
            }]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 60, 24);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
Create new card                                             
                                                            
┌Column Id:────────────────────────────────────────────────┐
│                         < Todo >                         │
└──────────────────────────────────────────────────────────┘
┌Card Title:───────────────────────────────────────────────┐
│|                                                         │
└──────────────────────────────────────────────────────────┘
┌Card Description:─────────────────────────────────────────┐
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
└──────────────────────────────────────────────────────────┘
┌Status:───────────────────────────────────────────────────┐
│                        < Active >                        │
└──────────────────────────────────────────────────────────┘
┌Blocked Reason:───────────────────────────────────────────┐
│|                                                         │
└──────────────────────────────────────────────────────────┘";

        assert_rendered_output(&rendered, expected_output);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_confirm_dialog_for_card_delete() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![Card {
                    id: String::from("card-1"),
                    column_id: String::from("column-1"),
                    title: String::from("Fix bug"),
                    description: Some(String::from("Desc")),
                    status: String::from("Active"),
                    blocked_reason: None,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                    started_at: None,
                    completed_at: None,
                }],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteCard,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };
        state.board_columns[0].list_state.select(Some(0));

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 80, 6);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
          ┌Do you want to delete selected card -> Fix bug ?──────────┐          
          │  YES                                                     │          
          │> NO                                                      │          
          │                                                          │          
          │                                                          │          
          └──────────────────────────────────────────────────────────┘          ";

        assert_rendered_output(&rendered, expected_output);
        Ok(())
    }

    #[test]
    fn test_manage_card_form_state_submits_when_title_populated_from_column_id() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);
        fill_text_input(&mut state.title, "Fix bug");

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        match result {
            ControlFlow::Break((id, column_id, title, description, status, blocked_reason)) => {
                assert!(id.is_none());
                assert_eq!(column_id, "column-1");
                assert_eq!(title, "Fix bug");
                assert_eq!(description, "");
                assert_eq!(status, CardStatus::Active);
                assert_eq!(blocked_reason, "");
            }
            _ => panic!("expected submission break"),
        }
    }

    #[tokio::test]
    async fn test_manage_board_details_state_confirm_dialog_no_returns_to_board_details()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteBoardColumn,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::BoardDetails));
        assert!(matches!(
            state.current_action_to_confirm,
            ActionToConfirm::DeleteBoardColumn
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_down_moves_card_selection() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![
                    Card {
                        id: String::from("card-1"),
                        column_id: String::from("column-1"),
                        title: String::from("First"),
                        description: Some(String::from("Desc")),
                        status: String::from("Active"),
                        blocked_reason: None,
                        created_at: Utc
                            .timestamp_opt(1_000_000, 0)
                            .single()
                            .unwrap()
                            .naive_utc(),
                        started_at: None,
                        completed_at: None,
                    },
                    Card {
                        id: String::from("card-2"),
                        column_id: String::from("column-1"),
                        title: String::from("Second"),
                        description: Some(String::from("Desc")),
                        status: String::from("Active"),
                        blocked_reason: None,
                        created_at: Utc
                            .timestamp_opt(1_000_000, 0)
                            .single()
                            .unwrap()
                            .naive_utc(),
                        started_at: None,
                        completed_at: None,
                    },
                ],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        assert!(state.board_columns[0].list_state.selected().is_none());

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();

        assert_eq!(state.board_columns[0].list_state.selected(), Some(0));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();

        assert_eq!(state.board_columns[0].list_state.selected(), Some(1));
        Ok(())
    }

    #[tokio::test]
    async fn test_current_window_start_returns_zero_when_total_columns_fit() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        assert_eq!(state.current_window_start(2, 4), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_current_window_start_clamps_window_at_end() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 4,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        assert_eq!(state.current_window_start(5, 4), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_cards_fetched_appends_cards()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let second_pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(second_pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let cards = vec![
            Card {
                id: String::from("card-1"),
                column_id: String::from("column-1"),
                title: String::from("First Card"),
                description: Some(String::from("Desc")),
                status: String::from("Active"),
                blocked_reason: None,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
                started_at: None,
                completed_at: None,
            },
            Card {
                id: String::from("card-2"),
                column_id: String::from("column-1"),
                title: String::from("Second Card"),
                description: Some(String::from("Desc")),
                status: String::from("Active"),
                blocked_reason: None,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
                started_at: None,
                completed_at: None,
            },
        ];

        state.handle_app_event(AppEvent::Card(CardEvent::CardsFetched(cards.clone())));

        assert_eq!(state.board_columns[0].cards.len(), 2);
        assert_eq!(state.board_columns[0].cards[0].title, "First Card");
        assert_eq!(state.board_columns[0].cards[1].title, "Second Card");
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_card_updated_replaces_card()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let second_pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let original_card = Card {
            id: String::from("card-1"),
            column_id: String::from("column-1"),
            title: String::from("Original Title"),
            description: Some(String::from("Desc")),
            status: String::from("Active"),
            blocked_reason: None,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
            started_at: None,
            completed_at: None,
        };
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(second_pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![original_card.clone()],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let updated_card = Card {
            id: String::from("card-1"),
            column_id: String::from("column-1"),
            title: String::from("Updated Title"),
            description: Some(String::from("Desc")),
            status: String::from("Active"),
            blocked_reason: None,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
            started_at: None,
            completed_at: None,
        };

        state.handle_app_event(AppEvent::Card(CardEvent::CardUpdated(updated_card)));

        assert_eq!(state.board_columns[0].cards.len(), 1);
        assert_eq!(state.board_columns[0].cards[0].title, "Updated Title");
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_manage_board_column_enter_creates_column()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir
            .path()
            .join("manage_board_details_create_column.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (card_service, _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service,
            notification_service,
        );
        let _ = event_receiver.recv().await;

        state.page_view = PageView::ManageBoardColumn;
        state.manage_board_column_form_state.clear();
        fill_text_input(&mut state.manage_board_column_form_state.name, "New Column");
        state
            .manage_board_column_form_state
            .wip_limit
            .set_input_value(2)
            .unwrap();
        state
            .manage_board_column_form_state
            .position
            .set_input_value(1)
            .unwrap();

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        let event = event_receiver
            .recv()
            .await
            .expect("expected board column created event");
        assert!(matches!(
            event,
            AppEvent::Board(BoardEvent::BoardColumnCreated(_))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_manage_card_enter_creates_card()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_create_card.db");
        let (board_service, mut board_event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut card_event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service.clone(),
            notification_service,
        );
        let _ = board_event_receiver.recv().await;

        let create_column_handle =
            board_service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let created_event = board_event_receiver
            .recv()
            .await
            .expect("expected board column created event");
        create_column_handle
            .await
            .expect("BoardService task panicked");

        let created_column = match created_event {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => column,
            other => panic!("expected created event, got {other:?}"),
        };

        state.page_view = PageView::ManageCard;
        state.manage_card_form_state = ManageCardFormState::new(vec![created_column.clone()]);
        fill_text_input(&mut state.manage_card_form_state.title, "New Card");

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        let event = card_event_receiver
            .recv()
            .await
            .expect("expected card created event");
        assert!(matches!(event, AppEvent::Card(CardEvent::CardCreated(_))));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_esc_from_manage_board_column_returns_board_details()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageBoardColumn,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();

        assert!(matches!(state.page_view, PageView::BoardDetails));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_esc_from_manage_card_returns_board_details()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                Arc::new(BoardColumnRepo::new(Arc::new(pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageCard,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();

        assert!(matches!(state.page_view, PageView::BoardDetails));
        Ok(())
    }
}
