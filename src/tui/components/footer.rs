use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// The `Footer` struct represents the TUI footer component, displaying a message at the bottom of the UI.
#[derive(Debug, Clone)]
pub struct Footer {
    message: String,
}

impl Footer {
    /// Creates a new `Footer` component/widget with the given message.
    ///
    /// # Arguments
    /// * `message` - The message text to display in the footer.
    pub fn new(message: String) -> Self {
        Self { message }
    }

    /// Updates the `message`, the inner state of the component/widget.
    ///
    /// # Arguments
    /// * `message` - The new message text to set.
    pub fn set_message(&mut self, message: String) {
        self.message = message;
    }
}

impl Widget for &mut Footer {
    /// Renders the UI (visual interpretation) for the `Footer` component/widget.
    /// Displays the message in yellow with a top border.
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // create a footer as Paragraph widget
        let footer = Paragraph::new(self.message.clone())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::TOP));
        footer.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the `Footer::new` function sets the message correctly.
    #[test]
    fn test_footer_new_sets_message() {
        let footer = Footer::new("Hello, world!".to_string());
        assert_eq!(footer.message, "Hello, world!");
    }

    /// Tests that `Footer::set_message` updates the message as expected.
    #[test]
    fn test_footer_set_message_updates_message() {
        let mut footer = Footer::new("Initial".to_string());
        footer.set_message("Updated".to_string());
        assert_eq!(footer.message, "Updated");
    }

    // Rendering tests for TUI widgets are usually done with integration or snapshot tests,
    // but you can at least check that the Widget trait is implemented and doesn't panic.
    /// Tests that rendering the `Footer` widget does not panic.
    #[test]
    fn test_footer_widget_trait_render_does_not_panic() {
        use ratatui::{layout::Rect, prelude::Buffer};

        let footer = Footer::new("Test".to_string());
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);

        // Should not panic
        footer.clone().render(area, &mut buf);
    }

    /// Tests the visual output of the `Footer` widget, ensuring the top border and message are rendered correctly.
    #[test]
    fn test_footer_visual_output_with_top_border() {
        use ratatui::{layout::Rect, prelude::Buffer, style::Color};
        let test_message = "TestMsg";
        let text_length: u16 = test_message.len().try_into().unwrap();
        let footer = Footer::new(test_message.to_string());
        // Area with height 2: row 0 for border, row 1 for message
        let area = Rect::new(0, 0, text_length, 2);
        let mut buf = Buffer::empty(area);

        footer.clone().render(area, &mut buf);

        // Build the expected buffer manually
        let mut expected = Buffer::empty(area);

        // Row 0: top border (should be '─' for Borders::TOP)
        for x in 0..text_length {
            expected[(x, 0)]
                .set_symbol("─")
                .set_style(Style::default().fg(Color::Yellow));
        }

        // Row 1: message "TestMsg" in yellow
        for (i, ch) in test_message.chars().enumerate() {
            expected[(i as u16, 1)]
                .set_symbol(&ch.to_string())
                .set_style(Style::default().fg(Color::Yellow));
        }

        assert_eq!(buf, expected);
    }
}
