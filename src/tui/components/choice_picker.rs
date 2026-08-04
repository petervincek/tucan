use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, Paragraph, StatefulWidget, Widget},
};

/// `Choice<T>` struct will encapsulate the udnerlying data structure that we want to associate with the
/// choice and the label for the purpose of rendering the choice in the widget
#[derive(Debug, Clone)]
pub struct Choice<T> {
    pub label: String,
    pub data: T,
}

impl<T> Choice<T> {
    /// creates new `Choice<T>`
    pub fn new(label: &str, data: T) -> Self {
        Self {
            label: String::from(label),
            data,
        }
    }
}

/// Stateless Configuration Widget, created and dropped on every frame
/// no state data like indexes stored here, just data for visual/rendering needs
pub struct ChoicePicker<'a, T> {
    options: &'a [Choice<T>], // borrow the options instead of owning them
    // because this widget will be recreated with every frame prefer borrows
    layout: PickerLayout, // configuration to pick the right visualization of choices
    block: Option<Block<'a>>, // optional `Block` widget
}

impl<'a, T> ChoicePicker<'a, T> {
    /// creates new `ChoicePicker<T>`, prefer the references/borrows of data
    /// to safe memory as the widget will be dropped and recreated with every frame
    pub fn new(options: &'a [Choice<T>]) -> Self {
        Self {
            options,
            layout: PickerLayout::VerticalList, // default layput is vertical list
            block: Option::None,
        }
    }

    /// builder method to set the layout presentation style
    pub fn layout(mut self, layout: PickerLayout) -> Self {
        self.layout = layout;
        self
    }

    /// builder method fo allow adding a optional border block
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

/// PickerLayout represents a configuration option for `ChoicePicker` to pick
/// different rendering of provided choices
pub enum PickerLayout {
    VerticalList,       // vertical list of choices
    HorizontalCarousel, // horizontal inline layout showing the active item: '< Option A >'
}

/// `ChoicePickerState` represents the long lived data/state for the statefull `ChoicePicker` widget
#[derive(Debug, Clone)]
pub struct ChoicePickerState {
    pub selected_index: usize,
}

/// `ChoicePickerState` contains methods for creating and managing the `ChoicePicker` internal state
/// and position of selected choice
impl ChoicePickerState {
    pub fn new(initial_index: usize) -> Self {
        Self {
            selected_index: initial_index,
        }
    }

    pub fn next(&mut self, total_options: usize) {
        if total_options > 0 {
            self.selected_index = (self.selected_index + 1) % total_options;
        }
    }

    pub fn previous(&mut self, total_options: usize) {
        if total_options > 0 {
            if self.selected_index == 0 {
                self.selected_index = total_options - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }
}

impl<'a, T> StatefulWidget for ChoicePicker<'a, T> {
    type State = ChoicePickerState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        // if there are no options, render nothing or just the empty block frame
        if self.options.is_empty() {
            if let Some(block) = self.block {
                Widget::render(block, area, buf);
            }
            return;
        }

        match self.layout {
            PickerLayout::VerticalList => {
                // convert our choices into generic ListItems
                let items: Vec<ListItem> = self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, option)| {
                        // Prepend a cursor symbol if this option matches our state index
                        let prefix = if i == state.selected_index {
                            "> "
                        } else {
                            "  "
                        };
                        ListItem::new(format!("{}{}", prefix, option.label))
                    })
                    .collect();

                // build a standard Ratatui List widget
                let mut list = List::new(items).highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

                // wrap it in a optional block
                if let Some(block) = self.block {
                    list = list.block(block);
                }

                // We can delegate rendering to the built-in List widget's own state structure,
                // or let it render statically into our custom layout area.
                // For simplicity, we match the selected highlight natively via our prefix mapping.
                StatefulWidget::render(
                    list,
                    area,
                    buf,
                    &mut ratatui::widgets::ListState::default()
                        .with_selected(Some(state.selected_index)),
                );
            }
            PickerLayout::HorizontalCarousel => {
                // get the currently active choice text based on our state index
                let current_choice = &self.options[state.selected_index];

                // format the carousel display style: < Option Value >
                let carousel_text = format!("< {} >", current_choice.label);

                // build a paragraph widget to display it inline
                let mut paragraph = Paragraph::new(carousel_text)
                    .alignment(Alignment::Center) // Center alignment looks best for carousels
                    .style(Style::default().fg(Color::Gray));

                if let Some(block) = self.block {
                    paragraph = paragraph.block(block);
                }

                // paragraph is a stateless widget, so we can render it directly
                Widget::render(paragraph, area, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};

    use super::*;
    use ratatui::{
        buffer::Buffer,
        prelude::Rect,
        widgets::{Block, Borders},
    };

    #[test]
    fn choice_new_stores_label_and_data() {
        let choice = Choice::new("alpha", 42);

        assert_eq!(choice.label, "alpha");
        assert_eq!(choice.data, 42);
    }

    #[test]
    fn choice_picker_state_next_cycles_and_ignores_empty_options() {
        let mut state = ChoicePickerState::new(1);

        state.next(3);
        assert_eq!(state.selected_index, 2);

        state.next(3);
        assert_eq!(state.selected_index, 0);

        state.next(0);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn choice_picker_state_previous_wraps_and_ignores_empty_options() {
        let mut state = ChoicePickerState::new(0);

        state.previous(3);
        assert_eq!(state.selected_index, 2);

        state.previous(3);
        assert_eq!(state.selected_index, 1);

        state.previous(0);
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn choice_picker_render_shows_selected_prefix_for_options() {
        let options = [
            Choice::new("one", 1),
            Choice::new("two", 2),
            Choice::new("three", 3),
        ];
        let mut state = ChoicePickerState::new(1);
        let widget = ChoicePicker::new(&options);
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
  one               
> two               
  three             ";
        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn choice_picker_render_with_block_draws_border_and_preserves_selection() {
        let options = [Choice::new("A", 'a'), Choice::new("B", 'b')];
        let mut state = ChoicePickerState::new(0);
        let widget =
            ChoicePicker::new(&options).block(Block::default().title("Pick").borders(Borders::ALL));
        let area = Rect::new(0, 0, 12, 4);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌Pick──────┐
│> A       │
│  B       │
└──────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn choice_picker_render_horizontal_carousel_shows_current_choice() {
        let options = [
            Choice::new("one", 1),
            Choice::new("two", 2),
            Choice::new("three", 3),
        ];
        let mut state = ChoicePickerState::new(2);
        let widget = ChoicePicker::new(&options).layout(PickerLayout::HorizontalCarousel);
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "      < three >     ";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn choice_picker_render_horizontal_carousel_with_block_draws_border_and_content() {
        let options = [Choice::new("A", 'a'), Choice::new("B", 'b')];
        let mut state = ChoicePickerState::new(1);
        let widget = ChoicePicker::new(&options)
            .layout(PickerLayout::HorizontalCarousel)
            .block(Block::default().title("Pick").borders(Borders::ALL));
        let area = Rect::new(0, 0, 14, 3);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌Pick────────┐
│    < B >   │
└────────────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }

    #[test]
    fn choice_picker_render_empty_options_with_block_draws_only_border() {
        let options: [Choice<u8>; 0] = [];
        let mut state = ChoicePickerState::new(0);
        let widget = ChoicePicker::new(&options).block(Block::default().borders(Borders::ALL));
        let area = Rect::new(0, 0, 7, 3);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);

        let rendered_output = buffer_to_string(&buf);
        let expected_output = "
┌─────┐
│     │
└─────┘";

        assert_rendered_output(&rendered_output, expected_output);
    }
}
