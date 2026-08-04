use std::ops::ControlFlow;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Stylize},
    text::Text,
    widgets::{StatefulWidget, Widget},
};

use crate::{
    model::board::BoardColumn,
    tui::{
        components::{
            integer_input::{IntegerInput, IntegerRange},
            text_input::TextInput,
        },
        handler::include_delegated_event_controls,
        page::common::{Center, EventHandler},
    },
};

/// `ManageBoardColumFormField` represents the enum of form fields available for widget `ManageBoardColumnForm`
#[derive(Debug, Clone)]
pub enum ManageBoardColumnFormField {
    Name,
    WipLimit,
    Position,
}

/// `ManageBoardColumnFormState` represents the state for statefull widget `ManageBoardColumnForm`
#[derive(Debug, Clone)]
pub struct ManageBoardColumnFormState {
    pub id: Option<String>, // state that will hold the optional id of the board column for already existing data
    // form for update vs. form for create
    pub name: TextInput, // widget for capturing the Kanban Board Column Name
    pub wip_limit: IntegerInput, // widget for capturing the Kanban Board Column's WorkInProgress limit
    pub position: IntegerInput,  // widget for capturing the Kanban Board Column's position
    pub current_field: ManageBoardColumnFormField,
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
pub struct ManageBoardColumnForm {}

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
