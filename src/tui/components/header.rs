use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
};

/// The `Header` struct represents the TUI header component, displaying a title at the top of the UI.
#[derive(Debug, Clone)]
pub struct Header {
    title: String,
}

impl Header {
    /// Creates a new `Header` component/widget with the given title.
    ///
    /// # Arguments
    /// * `title` - The title text to display in the header.
    pub fn new(title: String) -> Self {
        Self { title }
    }

    /// Updates the `title`, the inner state of the component/widget.
    ///
    /// # Arguments
    /// * `title` - The new title text to set.
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

impl Widget for &mut Header {
    /// Renders the UI (visual interpretation) for the `Header` component/widget.
    /// Displays the title in bold red with a bottom border.
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // create the header as a Paragraph widget
        let header = Paragraph::new(self.title.clone())
            .style(Style::new().red().bold())
            .block(Block::default().borders(Borders::BOTTOM));
        header.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_new_sets_title() {
        let header = Header::new("Hello, world!".to_string());
        assert_eq!(header.title, "Hello, world!");
    }

    #[test]
    fn test_header_set_title_updates_title() {
        let mut header = Header::new("Initial".to_string());
        header.set_title("Updated".to_string());
        assert_eq!(header.title, "Updated");
    }

    // Rendering tests for TUI widgets are usually done with integration or snapshot tests,
    // but you can at least check that the Widget trait is implemented and doesn't panic.
    #[test]
    fn test_header_widget_trait_render_does_not_panic() {
        use ratatui::{layout::Rect, prelude::Buffer};

        let header = Header::new("Test".to_string());
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);

        // Should not panic
        header.clone().render(area, &mut buf);
    }

    #[test]
    fn test_header_visual_output_with_top_border() {
        use ratatui::{layout::Rect, prelude::Buffer, style::Color};
        let test_message = "TestMsg";
        let text_length: u16 = test_message.len().try_into().unwrap();
        let header = Header::new(test_message.to_string());
        // Area with height 2: row 0 for message, row 1 for border
        let area = Rect::new(0, 0, text_length, 2);
        let mut buf = Buffer::empty(area);

        header.clone().render(area, &mut buf);

        // Build the expected buffer manually
        let mut expected = Buffer::empty(area);

        // Row 1: bottom border (should be '─' for Borders::BOTTOM)
        for x in 0..text_length {
            expected[(x, 1)]
                .set_symbol("─")
                .set_style(Style::default().fg(Color::Red).bold());
        }

        // Row 0: message "TestMsg" in bold red
        for (i, ch) in test_message.chars().enumerate() {
            expected[(i as u16, 0)]
                .set_symbol(&ch.to_string())
                .set_style(Style::default().fg(Color::Red).bold());
        }

        assert_eq!(buf, expected);
    }
}
