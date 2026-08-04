use std::ops::ControlFlow;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::page::common::EventHandler;

/// `IntegerInput` struct represents integer widget for client to actually input integer for futher processing
#[derive(Debug, Clone)]
pub struct IntegerInput {
    input_value: i32,            // variable to hold the actual state
    range: Option<IntegerRange>, // optional integer range for validation/limit purposes
    focused: bool,               // for visual feedback
    label: Option<String>,       // optional label for this integer input field
}

fn validate_input_within_range(input_value: i32, range: &Option<IntegerRange>) -> Result<()> {
    // if the range is provided, check the initial value is within range
    if let Some(IntegerRange {
        lower_limit,
        upper_limit,
    }) = range
        && (input_value < *lower_limit || input_value > *upper_limit)
    {
        return Err(anyhow::Error::msg(format!(
            "initial value for input [{input_value}] is not within the range [{lower_limit}, {upper_limit}]"
        )));
    }
    Ok(())
}

impl IntegerInput {
    pub fn new(input_value: i32, range: Option<IntegerRange>, focused: bool) -> Result<Self> {
        validate_input_within_range(input_value, &range)?;
        Ok(Self {
            input_value,
            range,
            focused,
            label: None,
        })
    }

    pub fn get_input_value(&self) -> i32 {
        self.input_value
    }

    pub fn set_input_value(&mut self, new_value: i32) -> Result<()> {
        validate_input_within_range(new_value, &self.range)?;
        self.input_value = new_value;
        Ok(())
    }

    pub fn focused(&mut self, state: bool) {
        self.focused = state;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }
}

/// `IntegerRange` struct represents range for allowed integers
#[derive(Debug, Clone)]
pub struct IntegerRange {
    lower_limit: i32,
    upper_limit: i32,
}

impl IntegerRange {
    pub fn new(lower_limit: i32, upper_limit: i32) -> Result<Self> {
        if lower_limit > upper_limit {
            return Err(anyhow::Error::msg(format!(
                "lower limit [{lower_limit}] can not be more than upper limit [{upper_limit}]"
            )));
        }
        Ok(Self {
            lower_limit,
            upper_limit,
        })
    }
}

impl Widget for &IntegerInput {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // prepare focus border if necessary/focused
        let border_style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        // prepare the right block based on the optional label used for block widget
        let block = if let Some(label) = &self.label {
            Block::new()
                .borders(Borders::all())
                .border_style(border_style)
                .title(String::from(label))
        } else {
            Block::new()
                .borders(Borders::all())
                .border_style(border_style)
        };

        // get the input_value
        let current_integer_value = self.input_value;
        // render the widget as paragraph with the actual integer value
        let paragraph = Paragraph::new(format!(" ▲ {current_integer_value} ▼ "))
            .alignment(Alignment::Left)
            .block(block);
        paragraph.render(area, buf);
    }
}

impl EventHandler<Option<i32>, ()> for IntegerInput {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        vec![
            (
                Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())),
                String::from("Cancel/Exit input"),
            ),
            (
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())),
                String::from("Confirm input"),
            ),
            (
                Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty())),
                String::from("Increment the integer value"),
            ),
            (
                Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty())),
                String::from("Decrement the integer value"),
            ),
        ]
    }
    fn handle_event(
        &mut self,
        event: crossterm::event::Event,
    ) -> Result<ControlFlow<Option<i32>, ()>> {
        match event {
            // handle all the key events for our input widget
            Event::Key(key_event) => match (key_event.code, key_event.modifiers) {
                (KeyCode::Esc, _) => {
                    // return from the input widget, ESC is sort of cancelation of the update
                    // so do not return anything
                    return Ok(ControlFlow::Break(None));
                }
                (KeyCode::Enter, _) => {
                    // return from the input widget, ENTER is confirmation of the update
                    // so return the updated value from the input, so it can be used
                    return Ok(ControlFlow::Break(Some(self.input_value)));
                }
                (KeyCode::Up, _) => {
                    let current_value = self.input_value + 1;
                    self.set_input_value(current_value)?;
                }
                (KeyCode::Down, _) => {
                    let current_value = self.input_value - 1;
                    self.set_input_value(current_value)?;
                }
                _ => {
                    // ignore anything else for now
                }
            },
            // handle all the mouse events for our input widget
            Event::Mouse(_mouse_event) => {}
            _ => {}
        }
        Ok(ControlFlow::Continue(()))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};

    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{layout::Rect, prelude::Buffer};
    use std::ops::ControlFlow;

    #[test]
    fn test_integer_range_new_valid_range() {
        let range = IntegerRange::new(0, 10).unwrap();

        assert_eq!(range.lower_limit, 0);
        assert_eq!(range.upper_limit, 10);
    }

    #[test]
    fn test_integer_range_new_invalid_range() {
        let result = IntegerRange::new(10, 0);

        assert!(result.is_err());
    }

    #[test]
    fn test_integer_input_new_sets_initial_state() {
        let range = IntegerRange::new(1, 5).unwrap();
        let input = IntegerInput::new(3, Some(range), true).unwrap();

        assert_eq!(input.get_input_value(), 3);
        assert!(input.focused);
    }

    #[test]
    fn test_integer_input_new_rejects_out_of_range_value() {
        let range = IntegerRange::new(1, 5).unwrap();
        let result = IntegerInput::new(6, Some(range), false);

        assert!(result.is_err());
    }

    #[test]
    fn test_set_input_value_accepts_valid_value() {
        let range = IntegerRange::new(0, 3).unwrap();
        let mut input = IntegerInput::new(1, Some(range), false).unwrap();

        input.set_input_value(3).unwrap();
        assert_eq!(input.get_input_value(), 3);
    }

    #[test]
    fn test_set_input_value_rejects_invalid_value() {
        let range = IntegerRange::new(0, 3).unwrap();
        let mut input = IntegerInput::new(1, Some(range), false).unwrap();

        let result = input.set_input_value(4);
        assert!(result.is_err());
        assert_eq!(input.get_input_value(), 1);
    }

    #[test]
    fn test_focused_setter_updates_focus_state() {
        let range = IntegerRange::new(0, 5).unwrap();
        let mut input = IntegerInput::new(2, Some(range), false).unwrap();

        input.focused(true);
        assert!(input.focused);

        input.focused(false);
        assert!(!input.focused);
    }

    #[test]
    fn test_get_event_controls_contains_expected_bindings() {
        let range = IntegerRange::new(0, 5).unwrap();
        let input = IntegerInput::new(2, Some(range), false).unwrap();

        let controls = input.get_event_controls();
        let labels: Vec<String> = controls.into_iter().map(|(_, label)| label).collect();

        assert!(labels.contains(&String::from("Cancel/Exit input")));
        assert!(labels.contains(&String::from("Confirm input")));
        assert!(labels.contains(&String::from("Increment the integer value")));
        assert!(labels.contains(&String::from("Decrement the integer value")));
    }

    #[test]
    fn test_render_does_not_panic_and_shows_integer_value() {
        let range = IntegerRange::new(0, 5).unwrap();
        let input = IntegerInput::new(2, Some(range), true).unwrap();

        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        (&input).render(area, &mut buf);

        let rendered = buffer_to_string(&buf);
        assert!(rendered.contains("▲ 2 ▼"));
    }

    #[test]
    fn test_label_method_sets_label() {
        let input = IntegerInput::new(1, None, false)
            .unwrap()
            .label("Volume".to_string());

        assert_eq!(input.label.as_deref(), Some("Volume"));
        assert_eq!(input.get_input_value(), 1);
    }

    #[test]
    fn test_render_with_label_shows_block_title_and_value() {
        let input = IntegerInput::new(4, None, true)
            .unwrap()
            .label("Count".to_string());

        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        (&input).render(area, &mut buf);

        let rendered = buffer_to_string(&buf);
        let expected_output = "
┌Count─────────────┐
│ ▲ 4 ▼            │
└──────────────────┘";
        assert_rendered_output(&rendered, expected_output);
    }

    #[test]
    fn test_handle_event_up_increments_value() {
        let range = IntegerRange::new(0, 5).unwrap();
        let mut input = IntegerInput::new(2, Some(range), false).unwrap();

        let result = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Up,
            KeyModifiers::empty(),
        )));

        assert!(result.is_ok());
        assert_eq!(input.get_input_value(), 3);
        assert!(matches!(result.unwrap(), ControlFlow::Continue(())));
    }

    #[test]
    fn test_handle_event_down_decrements_value() {
        let range = IntegerRange::new(0, 5).unwrap();
        let mut input = IntegerInput::new(2, Some(range), false).unwrap();

        let result = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::empty(),
        )));

        assert!(result.is_ok());
        assert_eq!(input.get_input_value(), 1);
        assert!(matches!(result.unwrap(), ControlFlow::Continue(())));
    }

    #[test]
    fn test_handle_event_up_above_upper_bound_returns_error() {
        let range = IntegerRange::new(0, 3).unwrap();
        let mut input = IntegerInput::new(3, Some(range), false).unwrap();

        let result = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Up,
            KeyModifiers::empty(),
        )));

        assert!(result.is_err());
        assert_eq!(input.get_input_value(), 3);
    }

    #[test]
    fn test_handle_event_down_below_lower_bound_returns_error() {
        let range = IntegerRange::new(0, 3).unwrap();
        let mut input = IntegerInput::new(0, Some(range), false).unwrap();

        let result = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::empty(),
        )));

        assert!(result.is_err());
        assert_eq!(input.get_input_value(), 0);
    }

    #[test]
    fn test_handle_event_esc_and_enter() {
        let range = IntegerRange::new(0, 5).unwrap();
        let mut input = IntegerInput::new(2, Some(range), false).unwrap();

        let result = input
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::empty(),
            )))
            .unwrap();
        assert!(matches!(result, ControlFlow::Break(None)));

        let mut input2 =
            IntegerInput::new(4, Some(IntegerRange::new(0, 5).unwrap()), false).unwrap();
        let result2 = input2
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::empty(),
            )))
            .unwrap();
        assert!(matches!(result2, ControlFlow::Break(Some(4))));
    }
}
