use std::ops::ControlFlow;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::page::common::EventHandler;

/// The `TextInput` struct represents a single-line text input widget with cursor, selection, and clipboard support.
#[derive(Debug, Clone)]
pub struct TextInput {
    buffer: String,         // string buffer to hold the state for the input
    cursor_position: usize, // track the cursor position
    cursor: char,
    selection_start: Option<usize>, // None if no selection
    clipboard: Option<String>,      // For copy/paste
    focused: bool,                  // For visual feedback
    label: Option<String>,          // optional label for this text input field
}

impl TextInput {
    /// Creates a new `TextInput` widget initialized with the given text.
    ///
    /// # Arguments
    /// * `text` - The initial text to populate the input buffer.
    pub fn new(text: String) -> Self {
        let length = text.chars().count();
        TextInput {
            buffer: text,
            cursor_position: length,
            cursor: '|',
            selection_start: None,
            clipboard: None,
            focused: true,
            label: None,
        }
    }

    /// Returns the current text as a borrowed string slice.
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// Returns an owned copy of the current input text.
    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    fn char_count(&self) -> usize {
        self.buffer.chars().count()
    }

    fn char_to_byte_index(&self, char_idx: usize) -> usize {
        if char_idx >= self.char_count() {
            self.buffer.len()
        } else {
            self.buffer
                .char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(self.buffer.len())
        }
    }

    fn remove_char_at(&mut self, char_idx: usize) {
        if char_idx >= self.char_count() {
            return;
        }
        let start = self.char_to_byte_index(char_idx);
        let end = self.char_to_byte_index(char_idx + 1);
        self.buffer.replace_range(start..end, "");
    }

    fn slice_by_char_range(&self, start: usize, end: usize) -> &str {
        let start_byte = self.char_to_byte_index(start);
        let end_byte = self.char_to_byte_index(end);
        &self.buffer[start_byte..end_byte]
    }

    /// Sets whether the widget is focused.
    pub fn set_focused(&mut self, state: bool) {
        self.focused = state;
    }

    /// Returns whether the widget is currently focused.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Deprecated alias for compatibility.
    pub fn focused(&mut self, state: bool) {
        self.set_focused(state);
    }

    pub fn label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    fn selected_range(&self) -> Option<(usize, usize)> {
        self.selection_start.and_then(|start| {
            let (a, b) = (
                start.min(self.cursor_position),
                start.max(self.cursor_position),
            );
            if a != b { Some((a, b)) } else { None }
        })
    }

    fn delete_selected_text(&mut self) -> bool {
        if let Some((start, end)) = self.selected_range() {
            let start_byte = self.char_to_byte_index(start);
            let end_byte = self.char_to_byte_index(end);
            self.buffer.replace_range(start_byte..end_byte, "");
            self.cursor_position = start;
            self.selection_start = None;
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_position = 0;
        self.selection_start = None;
        self.focused = true;
    }
}

impl Widget for &TextInput {
    /// Renders the `TextInput` widget, displaying the buffer and cursor, with a highlighted border if focused.
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
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

        let selection = self.selected_range();
        let inner_width = area.width.saturating_sub(2) as usize;

        let normal_style = Style::default();
        let selection_style = Style::default()
            .fg(Color::Black)
            .bg(Color::LightBlue)
            .add_modifier(Modifier::BOLD);
        let cursor_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let mut styled_chars: Vec<(char, Style)> = Vec::new();
        for (idx, ch) in self.buffer.chars().enumerate() {
            if idx == self.cursor_position {
                styled_chars.push((self.cursor, cursor_style));
            }
            let style = if let Some((start, end)) = selection {
                if idx >= start && idx < end {
                    selection_style
                } else {
                    normal_style
                }
            } else {
                normal_style
            };
            styled_chars.push((ch, style));
        }
        if self.cursor_position == self.char_count() {
            styled_chars.push((self.cursor, cursor_style));
        }

        let visible_chars = if styled_chars.len() <= inner_width {
            styled_chars
        } else {
            let cursor_display_pos = self.cursor_position;
            let max_start = styled_chars.len().saturating_sub(inner_width);
            let mut start = cursor_display_pos.saturating_sub(inner_width / 2);
            if start > max_start {
                start = max_start;
            }
            styled_chars[start..start + inner_width].to_vec()
        };

        let mut spans = Vec::new();
        let mut current_style = None;
        let mut current_text = String::new();

        for (ch, style) in visible_chars {
            if current_style.is_none() {
                current_style = Some(style);
                current_text.push(ch);
                continue;
            }
            if Some(style) == current_style {
                current_text.push(ch);
            } else {
                spans.push(Span::styled(current_text.clone(), current_style.unwrap()));
                current_text.clear();
                current_text.push(ch);
                current_style = Some(style);
            }
        }
        if !current_text.is_empty() {
            spans.push(Span::styled(
                current_text,
                current_style.unwrap_or(normal_style),
            ));
        }

        let paragraph = Paragraph::new(Text::from(Line::from(spans)))
            .alignment(Alignment::Left)
            .block(block);
        paragraph.render(area, buf);
    }
}

impl EventHandler<Option<String>, ()> for TextInput {
    /// Returns a vector of (Event, description) tuples for navigation/help display for the input widget.
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())),
            String::from("Cancel/Exit input"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())),
            String::from("Confirm input"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty())),
            String::from("Move cursor left"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)),
            String::from("Select left"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty())),
            String::from("Move cursor right"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT)),
            String::from("Select right"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty())),
            String::from("Delete character before cursor"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Delete, KeyModifiers::empty())),
            String::from("Delete character after cursor"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty())),
            String::from("Move cursor to start"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::empty())),
            String::from("Move cursor to end"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            String::from("Copy selection"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL)),
            String::from("Paste clipboard"),
        ));
        event_controls.push((
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())),
            String::from("Type character"),
        ));
        event_controls
    }

    /// Handles keyboard events for the input widget, supporting navigation, editing, selection, and clipboard.
    ///
    /// # Arguments
    /// * `event` - The input event to handle.
    ///
    /// # Returns
    /// * `ControlFlow` indicating whether to continue editing, cancel, or return the input value.
    fn handle_event(
        &mut self,
        event: crossterm::event::Event,
    ) -> Result<ControlFlow<Option<String>>> {
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
                    return Ok(ControlFlow::Break(Some(self.buffer.clone())));
                }
                (KeyCode::Left, _) => {
                    if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                        if self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_position);
                        }
                    } else {
                        self.selection_start = None;
                    }
                    // move the cursor to the left (if the move is possible)
                    if self.cursor_position > 0 {
                        self.cursor_position = self.cursor_position - 1;
                    }
                }
                (KeyCode::Right, _) => {
                    if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                        if self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_position);
                        }
                    } else {
                        self.selection_start = None;
                    }
                    // move the cursor to the right (if the move is possible)
                    if self.cursor_position < self.char_count() {
                        self.cursor_position = self.cursor_position + 1;
                    }
                }
                (KeyCode::Backspace, _) => {
                    if self.delete_selected_text() {
                        return Ok(ControlFlow::Continue(()));
                    }
                    // remove/delete the character before the cursor position (if possible)
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.remove_char_at(self.cursor_position);
                    }
                }
                (KeyCode::Delete, _) => {
                    if self.delete_selected_text() {
                        return Ok(ControlFlow::Continue(()));
                    }
                    // remove/delete the character after the cursor position (if possible)
                    if self.cursor_position < self.char_count() {
                        self.remove_char_at(self.cursor_position);
                    }
                }
                (KeyCode::Home, _) => {
                    self.cursor_position = 0;
                }
                (KeyCode::End, _) => {
                    self.cursor_position = self.buffer.len();
                }
                (KeyCode::Char(c), modifiers) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    // Accept any typed character, including uppercase, shifted punctuation, and Alt-modified input.
                    self.delete_selected_text();
                    let insert_byte = self.char_to_byte_index(self.cursor_position);
                    self.buffer.insert_str(insert_byte, &c.to_string());
                    self.cursor_position += 1;
                }
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    if let Some(start) = self.selection_start {
                        let (a, b) = (
                            start.min(self.cursor_position),
                            start.max(self.cursor_position),
                        );
                        if a != b {
                            self.clipboard = Some(self.slice_by_char_range(a, b).to_string());
                        }
                    }
                }
                (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                    if let Some(ref clip) = self.clipboard {
                        let insert_byte = self.char_to_byte_index(self.cursor_position);
                        self.buffer.insert_str(insert_byte, clip);
                        self.cursor_position += clip.chars().count();
                    }
                }
                _ => {
                    // ignore anything else for now
                }
            },
            _ => {}
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
    use std::ops::ControlFlow;

    #[test]
    fn test_text_input_new_sets_initial_state() {
        let input = TextInput::new("hello".to_string());
        assert_eq!(input.buffer, "hello");
        assert_eq!(input.cursor_position, 5);
        assert_eq!(input.cursor, '|');
        assert!(input.selection_start.is_none());
        assert!(input.clipboard.is_none());
        assert!(input.focused);
    }

    #[test]
    fn test_text_input_render_does_not_panic() {
        use ratatui::{layout::Rect, prelude::Buffer};
        let input = TextInput::new("test".to_string());
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        (&input).render(area, &mut buf);
    }

    #[test]
    fn test_handle_event_insert_char() {
        let mut input = TextInput::new("abc".to_string());
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('d'),
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "abcd");
        assert_eq!(input.cursor_position, 4);
    }

    #[test]
    fn test_handle_event_backspace() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor_position = 2;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "ac");
        assert_eq!(input.cursor_position, 1);
    }

    #[test]
    fn test_handle_event_delete() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor_position = 1;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Delete,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "ac");
        assert_eq!(input.cursor_position, 1);
    }

    #[test]
    fn test_handle_event_left_right() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor_position = 2;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Left,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.cursor_position, 1);
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.cursor_position, 2);
    }

    #[test]
    fn test_handle_event_home_end() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor_position = 2;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Home,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.cursor_position, 0);
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::End,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.cursor_position, 3);
    }

    #[test]
    fn test_handle_event_copy_paste() {
        let mut input = TextInput::new("abcdef".to_string());
        input.cursor_position = 2;
        input.selection_start = Some(0);
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(input.clipboard, Some("ab".to_string()));
        input.cursor_position = 6;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('v'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(input.buffer, "abcdefab");
        assert_eq!(input.cursor_position, 8);
    }

    #[test]
    fn test_handle_event_esc_and_enter() {
        let mut input = TextInput::new("abc".to_string());
        // Esc should break with None
        let result = input
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::empty(),
            )))
            .unwrap();
        match result {
            ControlFlow::Break(None) => {}
            _ => panic!("Expected ControlFlow::Break(None)"),
        }
        // Enter should break with Some(buffer)
        let mut input2 = TextInput::new("xyz".to_string());
        let result2 = input2
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::empty(),
            )))
            .unwrap();
        match result2 {
            ControlFlow::Break(Some(val)) => assert_eq!(val, "xyz"),
            _ => panic!("Expected ControlFlow::Break(Some)"),
        }
    }

    #[test]
    fn test_clear_resets_buffer_and_cursor_but_keeps_clipboard() {
        let mut input = TextInput::new("hello world".to_string());
        input.cursor_position = 5;
        input.selection_start = Some(2);
        input.clipboard = Some("hello".to_string());
        input.set_focused(false);

        input.clear();

        assert_eq!(input.buffer, "");
        assert_eq!(input.cursor_position, 0);
        assert!(input.selection_start.is_none());
        assert_eq!(input.clipboard, Some("hello".to_string()));
        assert!(input.is_focused());
    }

    #[test]
    fn test_set_focused_and_is_focused() {
        let mut input = TextInput::new("data".to_string());
        input.set_focused(false);
        assert!(!input.is_focused());
        input.set_focused(true);
        assert!(input.is_focused());
    }

    #[test]
    fn test_buffer_returns_borrowed_str() {
        let input = TextInput::new("hello".to_string());
        let buffer_str: &str = input.buffer();
        assert_eq!(buffer_str, "hello");
    }

    #[test]
    fn test_handle_event_insert_unicode_char_advances_cursor_by_one_char() {
        let mut input = TextInput::new("hi".to_string());
        input.cursor_position = 2;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('é'),
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "hié");
        assert_eq!(input.cursor_position, 3);
    }

    #[test]
    fn test_handle_event_backspace_removes_unicode_character() {
        let mut input = TextInput::new("hié".to_string());
        input.cursor_position = 3;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "hi");
        assert_eq!(input.cursor_position, 2);
    }

    #[test]
    fn test_render_clips_long_text_to_inner_width() {
        let mut input = TextInput::new("abcdefghij".to_string());
        input.cursor_position = 10;
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);

        (&input).render(area, &mut buf);
        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌────────┐
│defghij|│
└────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_render_centers_cursor_when_clipping() {
        let mut input = TextInput::new("abcdefghijklmnop".to_string());
        input.cursor_position = 5;
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);

        (&input).render(area, &mut buf);
        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌────────┐
│bcde|fgh│
└────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_render_centers_cursor_near_right_edge() {
        let mut input = TextInput::new("abcdefghijklmnop".to_string());
        input.cursor_position = 12;
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);

        (&input).render(area, &mut buf);
        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌────────┐
│ijkl|mno│
└────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_get_buffer_returns_current_text() {
        let input = TextInput::new("hello".to_string());
        assert_eq!(input.get_buffer(), "hello");
    }

    #[test]
    fn test_label_sets_optional_label() {
        let input = TextInput::new("text".to_string()).label("Name".to_string());
        assert_eq!(input.label, Some("Name".to_string()));
        assert_eq!(input.buffer, "text");
    }

    #[test]
    fn test_handle_event_left_shift_sets_selection_start() {
        let mut input = TextInput::new("hello".to_string());
        input.cursor_position = 3;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Left,
            KeyModifiers::SHIFT,
        )));
        assert_eq!(input.selection_start, Some(3));
        assert_eq!(input.cursor_position, 2);
    }

    #[test]
    fn test_handle_event_right_shift_sets_selection_start() {
        let mut input = TextInput::new("hello".to_string());
        input.cursor_position = 2;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::SHIFT,
        )));
        assert_eq!(input.selection_start, Some(2));
        assert_eq!(input.cursor_position, 3);
    }

    #[test]
    fn test_handle_event_copy_without_selection_does_not_set_clipboard() {
        let mut input = TextInput::new("abcdef".to_string());
        input.selection_start = Some(2);
        input.cursor_position = 2;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(input.clipboard.is_none());
    }

    #[test]
    fn test_handle_event_paste_without_clipboard_does_nothing() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor_position = 1;
        input.clipboard = None;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('v'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(input.buffer, "abc");
        assert_eq!(input.cursor_position, 1);
    }

    #[test]
    fn test_handle_event_backspace_at_start_does_nothing() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor_position = 0;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "abc");
        assert_eq!(input.cursor_position, 0);
    }

    #[test]
    fn test_handle_event_delete_at_end_does_nothing() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor_position = 3;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Delete,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "abc");
        assert_eq!(input.cursor_position, 3);
    }

    #[test]
    fn test_handle_event_insert_char_in_middle() {
        let mut input = TextInput::new("ace".to_string());
        input.cursor_position = 1;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "abce");
        assert_eq!(input.cursor_position, 2);
    }

    #[test]
    fn test_handle_event_insert_uppercase_char_with_shift() {
        let mut input = TextInput::new("hello".to_string());
        input.cursor_position = 5;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('A'),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(input.buffer, "helloA");
        assert_eq!(input.cursor_position, 6);
    }

    #[test]
    fn test_handle_event_insert_punctuation_with_shift() {
        let mut input = TextInput::new("count".to_string());
        input.cursor_position = 5;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('!'),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(input.buffer, "count!");
        assert_eq!(input.cursor_position, 6);
    }

    #[test]
    fn test_handle_event_insert_alt_char() {
        let mut input = TextInput::new("cafe".to_string());
        input.cursor_position = 4;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('é'),
            KeyModifiers::ALT,
        )));
        assert_eq!(input.buffer, "cafeé");
        assert_eq!(input.cursor_position, 5);
    }

    #[test]
    fn test_handle_event_insert_char_replaces_selected_text() {
        let mut input = TextInput::new("hello world".to_string());
        input.selection_start = Some(6);
        input.cursor_position = 11;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('!'),
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "hello !");
        assert_eq!(input.cursor_position, 7);
        assert!(input.selection_start.is_none());
    }

    #[test]
    fn test_handle_event_backspace_with_selection_deletes_selected_text() {
        let mut input = TextInput::new("hello world".to_string());
        input.selection_start = Some(5);
        input.cursor_position = 11;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, "hello");
        assert_eq!(input.cursor_position, 5);
        assert!(input.selection_start.is_none());
    }

    #[test]
    fn test_handle_event_delete_with_selection_deletes_selected_text() {
        let mut input = TextInput::new("hello world".to_string());
        input.selection_start = Some(0);
        input.cursor_position = 5;
        let _ = input.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Delete,
            KeyModifiers::empty(),
        )));
        assert_eq!(input.buffer, " world");
        assert_eq!(input.cursor_position, 0);
        assert!(input.selection_start.is_none());
    }

    #[test]
    fn test_get_event_controls_has_expected_entries() {
        let input = TextInput::new("".to_string());
        let controls = input.get_event_controls();
        assert_eq!(controls.len(), 13);
        assert_eq!(controls[0].1, "Cancel/Exit input");
        assert_eq!(controls[controls.len() - 1].1, "Type character");
    }

    #[test]
    fn test_render_does_not_panic_and_shows_cursor() {
        let input = TextInput::new("hey".to_string());
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);

        (&input).render(area, &mut buf);
        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌────────┐
│hey|    │
└────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_render_with_label_includes_title() {
        let input = TextInput::new("hello".to_string()).label("Name".to_string());
        let area = Rect::new(0, 0, 18, 3);
        let mut buf = Buffer::empty(area);

        (&input).render(area, &mut buf);
        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌Name────────────┐
│hello|          │
└────────────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_render_shows_cursor_at_expected_position() {
        let mut input = TextInput::new("rust".to_string());
        input.cursor_position = 2;
        let area = Rect::new(0, 0, 12, 3);
        let mut buf = Buffer::empty(area);

        (&input).render(area, &mut buf);
        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌──────────┐
│ru|st     │
└──────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_render_highlights_selected_text() {
        let mut input = TextInput::new("hello".to_string());
        input.selection_start = Some(1);
        input.cursor_position = 4;
        let area = Rect::new(0, 0, 12, 3);
        let mut buf = Buffer::empty(area);

        (&input).render(area, &mut buf);

        // selection covers the characters "ell" inside the input
        for x in 2..5 {
            assert_eq!(buf[(x, 1)].style().bg, Some(Color::LightBlue));
        }

        assert_ne!(buf[(1, 1)].style().bg, Some(Color::LightBlue));
        assert_ne!(buf[(5, 1)].style().bg, Some(Color::LightBlue));
    }
}
