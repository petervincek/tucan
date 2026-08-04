use crate::tui::page::manage_board_details::PageView;

/// `ActionToConfirm` represents all the action on thos page that needs to be confirmed before proceeding further
#[derive(Debug, Clone, PartialEq)]
pub enum ActionToConfirm {
    NoAction,
    DeleteBoardColumn,
    DeleteCard(PageView),
}
