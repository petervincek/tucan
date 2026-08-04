use std::ops::ControlFlow;

use anyhow::Result;

use chrono::{Local, NaiveDateTime};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::{
    model::{board::BoardColumn, card::Card},
    tui::{
        components::{
            choice_picker::{self, Choice, ChoicePicker, ChoicePickerState},
            date_time_picker::{DateTimePicker, DateTimePickerMode, DateTimePickerState},
            text_area::{TextArea, TextAreaState},
            text_input::TextInput,
        },
        handler::include_delegated_event_controls,
        page::common::{Center, EventHandler},
    },
};

/// `CardStatus` represents the possible values for card's status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardStatus {
    Active,
    Blocked,
}

/// `ManageCardFormField` represents the enum of form fields available for widget `ManageCardForm`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManageCardFormField {
    ColumnId,
    Title,
    Description,
    Status,
    BlockedReason,
}

pub type BoardColumnId = String;

/// `DateTimePickerTargetAttribute` represents the possible targets from the attributes maintained
/// by card form that can be set by Date/Time, currently it's just 'started_at' and 'completed_at'
/// which can be edited
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DateTimePickerTargetAttribute {
    StartedAt,
    CompletedAt,
}

/// `ManageCardFormPageView` represents possible page view states to render the specific functionality
/// to the user
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ManageCardFormPageView {
    Form,
    DateTimePicker {
        target_attribute: DateTimePickerTargetAttribute,
    },
}

/// `ManageCardFormState` represents the state for statefull widget `ManageCardForm`
#[derive(Debug, Clone)]
pub struct ManageCardFormState {
    pub id: Option<String>, // state that will hold the optional id of the card for already existing data
    // form for update vs. form for create
    pub column_id: (Vec<Choice<BoardColumnId>>, ChoicePickerState), // widget for capturing the association of Kanban Card with Kanban Board Column
    pub title: TextInput, // widget for capturing the title of the Kanban Card
    pub description: TextAreaState, // widget for capturing the main body (markup) of Kanban Card
    pub status: (Vec<Choice<CardStatus>>, ChoicePickerState), // widget for capturing the status of Kanban Card
    pub blocked_reason: TextInput, // widget for capturing the reason of Kanban Card being blocked
    pub date_time_picker: DateTimePickerState, // widget for capturing date/time values
    pub created_at: Option<NaiveDateTime>,
    pub started_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
    pub current_field: ManageCardFormField,
    pub current_page: ManageCardFormPageView,
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
            date_time_picker: DateTimePickerState::new(DateTimePickerMode::DateAndTime),
            created_at: None,
            started_at: None,
            completed_at: None,
            current_field: ManageCardFormField::ColumnId,
            current_page: ManageCardFormPageView::Form,
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
            created_at,
            started_at,
            completed_at,
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
                .map(|text| text.split("\n").map(|line| line.to_string()).collect())
                .unwrap_or_default(),
        );

        // status
        let status_choices = &self.status.0;
        let status_choice_picker_state = &mut self.status.1;
        status_choice_picker_state.selected_index = status_choices
            .iter()
            .enumerate()
            .find(|(_index, choice)| choice.label == status)
            .unwrap()
            .0;

        // blocked reason
        self.blocked_reason = TextInput::new(blocked_reason.unwrap_or_default())
            .label(String::from("Blocked Reason:"));
        self.blocked_reason.focused(false);

        // created_at
        self.created_at = Some(created_at);

        // started_at
        self.started_at = started_at;

        // completed_at
        self.completed_at = completed_at;

        self.current_field = ManageCardFormField::ColumnId;
        self.current_page = ManageCardFormPageView::Form;
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

        // reset date/time state fields when clearing the form
        self.created_at = None;
        self.started_at = None;
        self.completed_at = None;
        self.date_time_picker
            .preset_with_date_time(Local::now().naive_local());

        self.current_field = ManageCardFormField::ColumnId;
        self.current_page = ManageCardFormPageView::Form;
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
            Option<NaiveDateTime>,
            Option<NaiveDateTime>,
            Option<NaiveDateTime>,
        ),
        (),
    > for ManageCardFormState
{
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        if self.current_page == ManageCardFormPageView::Form {
            event_controls.push((
                Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
                String::from("Move To Next Field"),
            ));
            event_controls.push((
                Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)),
                String::from("Toggle date picker (started_at)"),
            ));
            event_controls.push((
                Event::Key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)),
                String::from("Toggle date picker (completed_at)"),
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
        } else if let ManageCardFormPageView::DateTimePicker {
            target_attribute: _,
        } = &self.current_page
        {
            event_controls.push((
                Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)),
                String::from("Toggle date picker (started_at)"),
            ));
            event_controls.push((
                Event::Key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)),
                String::from("Toggle date picker (completed_at)"),
            ));
            include_delegated_event_controls(
                &mut event_controls,
                self.date_time_picker.get_event_controls(),
            );
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
                Option<NaiveDateTime>,
                Option<NaiveDateTime>,
                Option<NaiveDateTime>,
            ),
            (),
        >,
    > {
        if self.current_page == ManageCardFormPageView::Form {
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
                // created_at
                let created_at = self.created_at;
                let started_at = self.started_at;
                let completed_at = self.completed_at;

                // self.clear();
                return Ok(ControlFlow::Break((
                    id,
                    column_id,
                    title,
                    description,
                    status,
                    blocked_reason,
                    created_at,
                    started_at,
                    completed_at,
                )));
            } else {
                if let Event::Key(key_event) = event
                    && (key_event.code == KeyCode::Char('s')
                        && key_event.modifiers == KeyModifiers::CONTROL)
                {
                    self.current_page = ManageCardFormPageView::DateTimePicker {
                        target_attribute: DateTimePickerTargetAttribute::StartedAt,
                    };
                    if let Some(date_time) = self.started_at {
                        self.date_time_picker.preset_with_date_time(date_time);
                    }
                } else if let Event::Key(key_event) = event
                    && (key_event.code == KeyCode::Char('f')
                        && key_event.modifiers == KeyModifiers::CONTROL)
                {
                    self.current_page = ManageCardFormPageView::DateTimePicker {
                        target_attribute: DateTimePickerTargetAttribute::CompletedAt,
                    };
                    if let Some(date_time) = self.completed_at {
                        self.date_time_picker.preset_with_date_time(date_time);
                    }
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
                                        let _result =
                                            blocked_reason_input_text.handle_event(event)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if let ManageCardFormPageView::DateTimePicker { target_attribute } =
            self.current_page.clone()
        {
            if let Event::Key(key_event) = event {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                        self.current_page = ManageCardFormPageView::Form;
                    }
                    (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                        self.current_page = ManageCardFormPageView::Form;
                    }
                    _ => {
                        // delegate the handling of the event to underlying widget 'date_time_picker'
                        let result = self.date_time_picker.handle_event(event)?;
                        // explicitly handle the break state, in that case set the returned value from the widget 'date_time_picker'
                        // to the requested 'target_attribute', this a sort of navigation mechanism
                        if let ControlFlow::Break(date_time) = result {
                            match target_attribute {
                                DateTimePickerTargetAttribute::StartedAt => {
                                    self.started_at = Some(date_time);
                                }
                                DateTimePickerTargetAttribute::CompletedAt => {
                                    self.completed_at = Some(date_time);
                                }
                            }
                            self.current_page = ManageCardFormPageView::Form;
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
pub struct ManageCardForm {}

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
        match state.current_page.clone() {
            ManageCardFormPageView::Form => {
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
                let column_id_border_style = if state.current_field == ManageCardFormField::ColumnId
                {
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
            ManageCardFormPageView::DateTimePicker {
                target_attribute: _,
            } => {
                let date_time_picker = DateTimePicker::new();
                StatefulWidget::render(date_time_picker, area, buf, &mut state.date_time_picker);
            }
        }
    }
}
