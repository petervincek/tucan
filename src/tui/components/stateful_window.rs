use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, StatefulWidget, Widget};

/// `StatefulWindow` represents the generic struct over the widget `W` we would like to
/// render inside that window and its state `S`
pub struct StatefulWindow<W, S>
// ensure W used the exact state type S
where
    W: StatefulWidget<State = S>,
{
    pub inner_widget: W,
    pub block: Block<'static>,
    // We use a PhantomData marker to tell Rust that S is logically part
    // of this struct, even though we don't store an instance of S here.
    // TODO: do we need this ???
    pub _marker: std::marker::PhantomData<S>,
}

impl<W, S> StatefulWindow<W, S>
where
    W: StatefulWidget<State = S>,
{
    pub fn new(title: &str, inner_widget: W) -> Self {
        Self {
            block: Block::default()
                .borders(Borders::ALL)
                .title(title.to_string()),
            inner_widget,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.block = self.block.borders(borders);
        self
    }

    pub fn title<T>(mut self, title: T) -> Self
    where
        T: Into<String>,
    {
        let title = title.into();
        self.block = self.block.title(title);
        self
    }

    pub fn title_style(mut self, style: Style) -> Self {
        self.block = self.block.title_style(style);
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.block = self.block.border_style(style);
        self
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = block;
        self
    }
}

impl<W, S> StatefulWidget for StatefulWindow<W, S>
where
    W: StatefulWidget<State = S>,
{
    type State = S;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        // Render the surrounding box
        // let block = Block::default().borders(Borders::ALL).title(self.title);

        let inner_area = self.block.inner(area);
        Widget::render(self.block, area, buf);

        // Render the inner widget, forwarding the mutable state slice down into it
        self.inner_widget.render(inner_area, buf, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Borders, List, ListItem, ListState, StatefulWidget};

    #[test]
    fn test_stateful_window_render_renders_border_title_and_inner_list_items() {
        let items = vec![ListItem::new("one"), ListItem::new("two")];
        let list = List::new(items);
        let mut state = ListState::default();
        state.select(Some(0));

        let window = StatefulWindow::new("My list", list);
        let area = Rect::new(0, 0, 12, 4);
        let mut buf = Buffer::empty(area);

        window.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌My list───┐
│one       │
│two       │
└──────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_stateful_window_render_uses_inner_area_for_list_widget() {
        let items = vec![ListItem::new("alpha"), ListItem::new("beta")];
        let list = List::new(items);
        let mut state = ListState::default();
        state.select(Some(1));

        let window = StatefulWindow::new("Items", list);
        let area = Rect::new(0, 0, 14, 5);
        let mut buf = Buffer::empty(area);

        window.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌Items───────┐
│alpha       │
│beta        │
│            │
└────────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_stateful_window_builder_chain_supports_title_and_style() {
        let items = vec![ListItem::new("one")];
        let list = List::new(items);
        let mut state = ListState::default();
        state.select(Some(0));

        let window = StatefulWindow::new("Init", list)
            .title("Updated")
            .title_style(Style::default().bold())
            .border_style(Style::default().fg(Color::Blue));

        let area = Rect::new(0, 0, 16, 3);
        let mut buf = Buffer::empty(area);

        window.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌Init─Updated──┐
│one           │
└──────────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_stateful_window_builder_supports_borders_modifier() {
        let items = vec![ListItem::new("one")];
        let list = List::new(items);
        let mut state = ListState::default();
        state.select(Some(0));

        let window = StatefulWindow::new("Init", list).borders(Borders::TOP);
        let area = Rect::new(0, 0, 12, 2);
        let mut buf = Buffer::empty(area);

        window.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
Init────────
one         ";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn test_stateful_window_builder_supports_block_override() {
        let items = vec![ListItem::new("X")];
        let list = List::new(items);
        let mut state = ListState::default();
        state.select(Some(0));

        let custom_block = Block::default().title("Custom").borders(Borders::TOP);
        let window = StatefulWindow::new("Init", list).block(custom_block);
        let area = Rect::new(0, 0, 10, 2);
        let mut buf = Buffer::empty(area);

        window.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
Custom────
X         ";

        assert_rendered_output(&rendered_output, expected_output);
    }
}
