use std::ops::ControlFlow;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::Style,
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::tui::page::common::EventHandler;

/// `TextAreaState` represents all the data needed for the stateless widget `TextArea`
#[derive(Debug)]
pub struct TextAreaState {
    title: String,
    focused: bool,
    text: Vec<String>,
    cursor: (usize, usize),
    selection_anchor: Option<(usize, usize)>,
    yank_text: String,
}

impl TextAreaState {
    pub fn new(title: String) -> Self {
        Self {
            title,
            focused: false,
            text: vec![String::new()],
            cursor: (0, 0),
            selection_anchor: None,
            yank_text: String::new(),
        }
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn set_text(&mut self, text: Vec<String>) {
        self.text = text;
        self.cursor = (0, 0);
        self.selection_anchor = None;
        self.yank_text.clear();
    }

    pub fn get_text(&self) -> Vec<String> {
        self.text.clone()
    }

    pub fn get_text_as_string(&self) -> String {
        self.get_text().join("\n")
    }

    pub fn set_cursor(&mut self, cursor: (usize, usize)) {
        self.cursor = cursor;
        self.selection_anchor = None;
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        self.cursor
    }

    fn restore_textarea_state(&self, textarea: &mut tui_textarea::TextArea<'static>) {
        textarea.set_lines(self.get_text(), self.cursor);
        textarea.set_yank_text(self.yank_text.clone());

        if let Some(anchor) = self.selection_anchor {
            if anchor != self.cursor {
                textarea.set_lines(self.get_text(), anchor);
                textarea.start_selection();
                let (row, col) = self.cursor;
                textarea.move_cursor(tui_textarea::CursorMove::Jump(
                    row.min(u16::MAX as usize) as u16,
                    col.min(u16::MAX as usize) as u16,
                ));
            } else {
                textarea.start_selection();
            }
        }
    }
}

/// `TextArea` represents stateless widget data structure
pub struct TextArea {}

impl TextArea {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for TextArea {
    type State = TextAreaState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let text_area_widget = &mut tui_textarea::TextArea::default();
        state.restore_textarea_state(text_area_widget);
        let border_style = if state.focused {
            Style::default().fg(ratatui::style::Color::Yellow)
        } else {
            Style::default().fg(ratatui::style::Color::Gray)
        };
        text_area_widget.set_block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(state.title.clone())
                .title_style(Style::default().bold()),
        );
        text_area_widget.render(area, buf);
    }
}

impl EventHandler<Option<String>, ()> for TextAreaState {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = vec![];
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            String::from("Go Back"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            String::from("Copy selection"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            String::from("Cut selection"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL)),
            String::from("Paste"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)),
            String::from("Select left"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT)),
            String::from("Select right"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT)),
            String::from("Select up"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT)),
            String::from("Select down"),
        ));
        event_controls
    }

    fn handle_event(&mut self, event: Event) -> Result<ControlFlow<Option<String>, ()>> {
        if let Event::Key(key_event) = event {
            if key_event.code == KeyCode::Esc && key_event.modifiers == KeyModifiers::NONE {
                return Ok(ControlFlow::Break(None));
            }

            let mut text_area_widget = tui_textarea::TextArea::default();
            self.restore_textarea_state(&mut text_area_widget);

            let old_cursor = self.cursor;
            let modified = match (key_event.code, key_event.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    text_area_widget.copy();
                    false
                }
                (KeyCode::Char('x'), KeyModifiers::CONTROL) => text_area_widget.cut(),
                (KeyCode::Char('v'), KeyModifiers::CONTROL) => text_area_widget.paste(),
                _ => text_area_widget.input(key_event),
            };

            if modified {
                self.text = text_area_widget.lines().iter().cloned().collect();
            }
            self.cursor = text_area_widget.cursor();
            self.yank_text = text_area_widget.yank_text();
            self.selection_anchor = if text_area_widget.selection_range().is_some() {
                self.selection_anchor.or(Some(old_cursor))
            } else {
                None
            };
        }

        Ok(ControlFlow::Continue(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{buffer::Buffer, prelude::Rect};

    #[test]
    fn test_text_area_state_new_defaults() {
        let state = TextAreaState::new(String::from("Message Payload:"));

        assert_eq!(state.get_title(), "Message Payload:");
        assert!(!state.is_focused());
        assert_eq!(state.get_text(), vec![String::new()]);
        assert_eq!(state.get_cursor(), (0, 0));
    }

    #[test]
    fn test_text_area_handle_event_inserts_character_and_moves_cursor() {
        let mut state = TextAreaState::new(String::from("Payload"));
        state.set_text(vec![String::from("hello")]);
        state.set_cursor((0, 5));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('!'),
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert_eq!(state.get_text(), vec![String::from("hello!")]);
        assert_eq!(state.get_cursor(), (0, 6));
    }

    #[test]
    fn test_text_area_handle_event_selection_copy_cut_paste() {
        let mut state = TextAreaState::new(String::from("Payload"));
        state.set_text(vec![String::from("hello")]);
        state.set_cursor((0, 0));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Right,
                KeyModifiers::SHIFT,
            )))
            .unwrap();
        assert_eq!(state.selection_anchor, Some((0, 0)));
        assert_eq!(state.get_cursor(), (0, 1));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        assert_eq!(state.yank_text, "h");
        assert_eq!(state.get_text(), vec![String::from("hello")]);

        state.set_cursor((0, 5));
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('v'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        assert_eq!(state.get_text(), vec![String::from("helloh")]);

        state.set_text(vec![String::from("hello")]);
        state.set_cursor((0, 0));
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Right,
                KeyModifiers::SHIFT,
            )))
            .unwrap();
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        assert_eq!(state.yank_text, "h");
        assert_eq!(state.get_text(), vec![String::from("ello")]);
        assert_eq!(state.get_cursor(), (0, 0));
    }

    #[test]
    fn test_text_area_get_event_controls_contains_clipboard_and_selection_keys() {
        let state = TextAreaState::new(String::from("Payload"));
        let controls = state.get_event_controls();

        let descriptions: Vec<String> = controls.into_iter().map(|(_, desc)| desc).collect();
        assert!(descriptions.contains(&String::from("Copy selection")));
        assert!(descriptions.contains(&String::from("Cut selection")));
        assert!(descriptions.contains(&String::from("Paste")));
        assert!(descriptions.contains(&String::from("Select left")));
        assert!(descriptions.contains(&String::from("Select right")));
    }

    #[test]
    fn test_text_area_render_default_widget_output() {
        let mut state = TextAreaState::new(String::from("Message Payload:"));
        let widget = TextArea::new();
        let area = Rect::new(0, 0, 35, 5);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered_output = buffer_to_string(&buf);
        let expected_output = "\
┌Message Payload:─────────────────┐\n\
│                                 │\n\
│                                 │\n\
│                                 │\n\
└─────────────────────────────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_text_area_render_with_text_lines_and_cursor() {
        let mut state = TextAreaState::new(String::from("Message Payload:"));
        state.set_text(vec![String::from("hello"), String::from("world")]);
        state.set_cursor((3, 0));
        let widget = TextArea::new();
        let area = Rect::new(0, 0, 35, 6);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered_output = buffer_to_string(&buf);
        let expected_output = "\
┌Message Payload:─────────────────┐\n\
│hello                            │\n\
│world                            │\n\
│                                 │\n\
│                                 │\n\
└─────────────────────────────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_text_area_get_text_as_string_single_line() {
        let mut state = TextAreaState::new(String::from("Payload"));
        state.set_text(vec![String::from("hello")]);

        assert_eq!(state.get_text_as_string(), "hello");
    }

    #[test]
    fn test_text_area_get_text_as_string_multiple_lines() {
        let mut state = TextAreaState::new(String::from("Payload"));
        state.set_text(vec![
            String::from("hello"),
            String::from("world"),
            String::new(),
        ]);

        assert_eq!(state.get_text_as_string(), "hello\nworld\n");
    }
}
