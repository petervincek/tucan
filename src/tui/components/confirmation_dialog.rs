use crate::tui::components::choice_picker::{Choice, ChoicePicker, ChoicePickerState};

/// `ChoicePicker` implementation for a confirmation dialog, as the confirmation dialog is
/// basically a choice picker with limited options to YES/NO, true/false
impl ChoicePicker<'static, bool> {
    pub fn confirmation_dialog() -> (Vec<Choice<bool>>, ChoicePickerState) {
        let options = vec![Choice::new("YES", true), Choice::new("NO", false)];
        let initial_state = ChoicePickerState::new(1);

        (options, initial_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_dialog_returns_two_options() {
        let (options, state) = ChoicePicker::confirmation_dialog();

        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "YES");
        assert_eq!(options[1].label, "NO");
        assert_eq!(options[0].data, true);
        assert_eq!(options[1].data, false);
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn confirmation_dialog_initial_state_selects_no() {
        let (_options, state) = ChoicePicker::confirmation_dialog();

        assert_eq!(state.selected_index, 1);
    }
}
