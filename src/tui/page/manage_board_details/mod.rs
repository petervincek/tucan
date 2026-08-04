mod confirm_dialog;
mod manage_board_column_form;
mod manage_card_form;
#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    ops::ControlFlow,
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::Result;
use chrono::NaiveDateTime;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};
use tracing::debug;

use crate::{
    core::config::AppConfig,
    model::{
        board::{BoardColumn, NewBoardColumn},
        card::{Card, NewCard},
    },
    tui::{
        components::{
            choice_picker::{self, Choice, ChoicePicker, ChoicePickerState},
            notification_panel::NotificationMessage,
            stateful_window::StatefulWindow,
        },
        event::events::{
            AppEvent::{self},
            BoardEvent, CardEvent,
        },
        handler::include_delegated_event_controls,
        page::{
            common::{Center, EventHandler},
            manage_board_details::{
                PageView::{BoardDetails, ViewCard},
                confirm_dialog::ActionToConfirm,
                manage_board_column_form::{ManageBoardColumnForm, ManageBoardColumnFormState},
                manage_card_form::{CardStatus, ManageCardForm, ManageCardFormState},
            },
        },
        service::{board::BoardService, card::CardService, notifications::NotificationService},
    },
};

/// `PageView` represents the specific view page (it's part of navigation) for `ManageBoardDetails` page
/// according to the current application state
#[derive(Debug, Clone, PartialEq)]
pub enum PageView {
    BoardDetails,
    ManageBoardColumn,
    ManageCard { coming_from: Box<PageView> },
    ViewCard,
    ConfirmDialog,
}

#[derive(Debug, Clone)]
struct BoardColumnWithCards {
    pub board_column: BoardColumn,
    pub cards: Vec<Card>,
    pub list_state: ListState,
}

impl BoardColumnWithCards {
    pub fn new(board_column: BoardColumn, cards: Vec<Card>) -> Self {
        Self {
            board_column,
            cards,
            list_state: ListState::default(),
        }
    }
}

type CardId = String;

/// `ManageBoardDetailsState` represents the state for statefull widget `ManageBoardDetails`
#[derive(Clone)]
pub struct ManageBoardDetailsState {
    app_config: Arc<Mutex<AppConfig>>,
    board_service: Arc<BoardService>,
    card_service: Arc<CardService>,
    notification_service: Arc<NotificationService>,
    // page state
    board_columns: Vec<BoardColumnWithCards>,
    card_to_view: Option<Card>,
    current_column_index: usize,
    page_view: PageView,
    current_action_to_confirm: ActionToConfirm,
    view_card_scroll_offset: u16,
    manage_board_column_form_state: ManageBoardColumnFormState,
    manage_card_form_state: ManageCardFormState,
    confirm_dialog_state: (Vec<Choice<bool>>, ChoicePickerState),
    marked_cards: HashMap<CardId, Card>,
}

impl ManageBoardDetailsState {
    fn current_window_start(&self, total_columns_num: usize, display_columns_num: usize) -> usize {
        if total_columns_num <= display_columns_num {
            return 0;
        }

        let half_window = display_columns_num / 2;
        let start = self.current_column_index.saturating_sub(half_window);

        if start + display_columns_num > total_columns_num {
            total_columns_num - display_columns_num
        } else {
            start
        }
    }

    pub fn new(
        app_config: Arc<Mutex<AppConfig>>,
        board_service: Arc<BoardService>,
        card_service: Arc<CardService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        // trigger the board columns fetching, fire-and-forget call
        board_service.list_columns();
        Self {
            app_config,
            board_service,
            card_service,
            notification_service,
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        }
    }

    pub fn handle_app_event(&mut self, app_event: AppEvent) {
        match app_event {
            AppEvent::Board(BoardEvent::BoardColumnCreated(created_board_column)) => {
                debug!("New Board Column created: {created_board_column:?}");
                let board_column_name = created_board_column.name;
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("New Board Column created, name: {board_column_name}"),
                        Instant::now(),
                    ));
                // load the board columns to reflect the changes
                self.board_service.list_columns();
            }
            AppEvent::Board(BoardEvent::BoardColumnFetched(fetched_board_column)) => {
                debug!("Board Column fetched: {fetched_board_column:?}");
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Board Column fetched: {fetched_board_column:?}"),
                        Instant::now(),
                    ));
            }
            AppEvent::Board(BoardEvent::BoardColumnsFetched(fetched_board_columns)) => {
                debug!("Board Columns fetched: {fetched_board_columns:?}");
                let board_columns_size = fetched_board_columns.len();
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Fetched {board_columns_size} board columns"),
                        Instant::now(),
                    ));
                self.board_columns = fetched_board_columns
                    // .clone()
                    .iter()
                    .map(|board_column| BoardColumnWithCards::new(board_column.clone(), vec![]))
                    .collect();
                self.manage_card_form_state
                    .set_board_columns(fetched_board_columns);
                if self.current_column_index >= self.board_columns.len() {
                    self.current_column_index = self.board_columns.len().saturating_sub(1);
                }
                // trigger the card loading/fetching
                self.board_columns
                    .iter()
                    .for_each(|board_column_with_cards| {
                        let column_id = &board_column_with_cards.board_column.id;
                        self.card_service.list_columns(column_id);
                    });
            }
            AppEvent::Board(BoardEvent::BoardColumnUpdated(updated_board_column)) => {
                debug!("Board Column updated: {updated_board_column:?}");
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Board Column updated: {updated_board_column:?}"),
                        Instant::now(),
                    ));
                // load the board columns to reflect the changes
                self.board_service.list_columns();
            }
            AppEvent::Board(BoardEvent::BoardColumnDeleted(deleted_board_column_id)) => {
                debug!("Board Column deleted: {deleted_board_column_id:?}");
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Board Column deleted: {deleted_board_column_id:?}"),
                        Instant::now(),
                    ));
                // load the board columns to reflect the changes
                self.board_service.list_columns();
            }
            AppEvent::Board(BoardEvent::Error(error)) => {
                self.notification_service
                    .send_notification(NotificationMessage::ErrorMsg(
                        format!("Error: {error}"),
                        Instant::now(),
                    ));
            }
            AppEvent::Config(_) => {}
            AppEvent::Card(CardEvent::CardCreated(created_card)) => {
                debug!("New Card created: {created_card:?}");
                let card_title = &created_card.title;
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("New Card created, title: {card_title}"),
                        Instant::now(),
                    ));
                let column_id = created_card.column_id.clone();
                for board_column_with_cards in &mut self.board_columns {
                    if board_column_with_cards.board_column.id == column_id {
                        board_column_with_cards.cards.push(created_card.clone());
                    }
                }
            }
            AppEvent::Card(CardEvent::CardFetched(fetched_card)) => {
                let card_id = &fetched_card.id;
                debug!("Card fetched");
                for board_column_with_cards in &mut self.board_columns {
                    board_column_with_cards
                        .cards
                        .retain(|card| card.id != *card_id);
                    if board_column_with_cards.board_column.id == fetched_card.column_id {
                        board_column_with_cards.cards.push(fetched_card.clone());
                    }
                }
            }
            AppEvent::Card(CardEvent::CardsFetched(fetched_cards)) => {
                debug!("Cards fetched");
                for card in fetched_cards {
                    let column_id = card.column_id.clone();
                    for board_column_with_cards in &mut self.board_columns {
                        if board_column_with_cards.board_column.id == column_id {
                            board_column_with_cards.cards.push(card.clone());
                        }
                    }
                }
            }
            AppEvent::Card(CardEvent::CardUpdated(updated_card)) => {
                let card_id = &updated_card.id;
                debug!("Card updated: {card_id}");
                for board_column_with_cards in &mut self.board_columns {
                    board_column_with_cards
                        .cards
                        .retain(|card| card.id != *card_id);
                    if board_column_with_cards.board_column.id == updated_card.column_id {
                        board_column_with_cards.cards.push(updated_card.clone());
                    }
                }
                self.card_to_view = Some(updated_card.clone());
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Card updated, id: {card_id}"),
                        Instant::now(),
                    ));
            }
            AppEvent::Card(CardEvent::CardDeleted(deleted_card)) => {
                debug!("Card deleted: {deleted_card}");
                for board_column_with_cards in &mut self.board_columns {
                    board_column_with_cards
                        .cards
                        .retain(|card| card.id != deleted_card);
                }
                self.notification_service
                    .send_notification(NotificationMessage::InfoMsg(
                        format!("Card deleted, id: {deleted_card}"),
                        Instant::now(),
                    ));
            }
            AppEvent::Card(CardEvent::Error(error)) => {
                self.notification_service
                    .send_notification(NotificationMessage::ErrorMsg(
                        format!("Error: {error}"),
                        Instant::now(),
                    ));
            }
        }
    }
}

impl EventHandler<(), ()> for ManageBoardDetailsState {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        let mut event_controls = Vec::new();
        match self.page_view {
            PageView::BoardDetails => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL)),
                    String::from("Create New Board Column"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL)),
                    String::from("Create New Card"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL)),
                    String::from("Edit Selected Board Column"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
                    String::from("Delete Selected Board Column"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
                    String::from("Remove Selected Card"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
                    String::from("(Un)Mark Selected Card"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
                    String::from("Move On The Right"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
                    String::from("Move On The Left"),
                ));
            }
            PageView::ManageBoardColumn => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back to Board List"),
                ));
                include_delegated_event_controls(
                    &mut event_controls,
                    self.manage_board_column_form_state.get_event_controls(),
                );
            }
            PageView::ConfirmDialog => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                    String::from("Move To The Previous Option"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
                    String::from("Move To The Previous Option"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                    String::from("Confirm selection"),
                ));
            }
            PageView::ManageCard { coming_from: _ } => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back to Board List"),
                ));
                include_delegated_event_controls(
                    &mut event_controls,
                    self.manage_card_form_state.get_event_controls(),
                );
            }
            PageView::ViewCard => {
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    String::from("Go Back to Board List"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                    String::from("Scroll Card Up"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
                    String::from("Scroll Card Down"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                    String::from("Scroll Card Up"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
                    String::from("Remove this card"),
                ));
                event_controls.push((
                    Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL)),
                    String::from("Edit this card"),
                ));
            }
        }
        event_controls
    }

    fn handle_event(&mut self, event: Event) -> Result<ControlFlow<(), ()>> {
        match &self.page_view {
            PageView::BoardDetails => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                            debug!("Handling the manage board column [create new column]");
                            self.manage_board_column_form_state.clear(); // clear the form
                            self.page_view = PageView::ManageBoardColumn;
                        }
                        (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                            debug!("Handling the manage card [create new card]");
                            self.manage_card_form_state.clear(); // clear the form
                            self.page_view = PageView::ManageCard {
                                coming_from: Box::new(PageView::BoardDetails),
                            };
                        }
                        (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                            debug!("Handling the manage board column [update existing column]");
                            self.manage_board_column_form_state.clear(); // clear the form
                            // preset the form with existing data from selected board column
                            let board_column = self.board_columns[self.current_column_index]
                                .board_column
                                .clone();
                            self.manage_board_column_form_state
                                .preset_with_board_column_data(board_column);
                            self.page_view = PageView::ManageBoardColumn;
                        }
                        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                            debug!("Handling deleting of selected column");
                            // we will allow the delete of column just in case there are no card available
                            // open the confirmation dialog asking to confirm the delete action for selected board column
                            self.current_action_to_confirm = ActionToConfirm::DeleteBoardColumn;
                            self.page_view = PageView::ConfirmDialog;
                        }
                        (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                            debug!(
                                "Handling removing of selected card from currently highlighted column"
                            );
                            let card_list_state =
                                self.board_columns[self.current_column_index].list_state;
                            if card_list_state.selected().is_some() {
                                self.current_action_to_confirm =
                                    ActionToConfirm::DeleteCard(BoardDetails);
                                self.page_view = PageView::ConfirmDialog;
                            }
                        }
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            debug!(
                                "Handling detail view of selected card from currently highlighted column"
                            );
                            let card_list_state =
                                self.board_columns[self.current_column_index].list_state;
                            if let Some(selected_card_index) = card_list_state.selected() {
                                let cards = &self.board_columns[self.current_column_index].cards;
                                self.card_to_view = Some(cards[selected_card_index].clone());
                                self.view_card_scroll_offset = 0;
                                self.page_view = PageView::ViewCard;
                            }
                        }
                        (KeyCode::Right, KeyModifiers::NONE) => {
                            if !self.board_columns.is_empty() {
                                debug!("Moving to the next column, moving right");
                                self.current_column_index =
                                    (self.current_column_index + 1) % self.board_columns.len();
                            }
                        }
                        (KeyCode::Left, KeyModifiers::NONE) => {
                            if !self.board_columns.is_empty() {
                                debug!("Moving to the previous column, moving left");
                                if self.current_column_index == 0 {
                                    self.current_column_index = self.board_columns.len() - 1;
                                } else {
                                    self.current_column_index -= 1;
                                }
                            }
                        }
                        (KeyCode::Up, KeyModifiers::NONE) => {
                            if !self.board_columns.is_empty() {
                                debug!("Moving up the cards in a column");
                                let selected_board_column =
                                    &mut self.board_columns[self.current_column_index];
                                let card_list_state = &mut selected_board_column.list_state;
                                card_list_state.select_previous();
                            }
                        }
                        (KeyCode::Down, KeyModifiers::NONE) => {
                            if !self.board_columns.is_empty() {
                                debug!("Moving down the cards in a column");
                                let selected_board_column =
                                    &mut self.board_columns[self.current_column_index];
                                let card_list_state = &mut selected_board_column.list_state;
                                card_list_state.select_next();
                            }
                        }
                        (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                            debug!(
                                "Handling toggling of marking of selected card from currently highlighted column"
                            );
                            let card_list_state =
                                self.board_columns[self.current_column_index].list_state;
                            if let Some(selected_card_index) = card_list_state.selected() {
                                let cards = &self.board_columns[self.current_column_index].cards;
                                let card_to_toggle = cards[selected_card_index].clone();
                                // build/maintain a list of marked cards
                                if self.marked_cards.contains_key(&card_to_toggle.id) {
                                    // remove the card from the marked ones
                                    self.marked_cards.remove(&card_to_toggle.id);
                                } else {
                                    // mark the card
                                    self.marked_cards
                                        .insert(card_to_toggle.id.clone(), card_to_toggle);
                                }
                            }
                        }
                        (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                            debug!(
                                "Handling moving of marked cards to currently highlighted column"
                            );
                            let board_column =
                                &self.board_columns[self.current_column_index].board_column;
                            for marked_card in self.marked_cards.values() {
                                let mut card_to_update = marked_card.clone();
                                card_to_update.column_id = board_column.id.clone();
                                self.card_service.update_card(card_to_update);
                            }
                        }
                        _ => {}
                    }
                }
            }
            PageView::ManageBoardColumn => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => {
                            self.page_view = PageView::BoardDetails
                        }
                        // delegate any other event to the create board form state
                        _ => {
                            let result = self.manage_board_column_form_state.handle_event(event)?;
                            if let ControlFlow::Break((
                                board_column_id,
                                board_column_name,
                                wip_limit,
                                board_column_position,
                            )) = result
                            {
                                // process the collected data from the form
                                if let Some(board_column_id) = board_column_id {
                                    // in this case the id already exists, so it's update
                                    self.board_service.update_column(BoardColumn {
                                        id: board_column_id,
                                        name: board_column_name,
                                        wip_limit: wip_limit.try_into().unwrap(),
                                        position: board_column_position.try_into().unwrap(),
                                        created_at: NaiveDateTime::default(), // does not matter here, it's not used
                                    });
                                } else {
                                    // in this case the id does not exist, so it's create
                                    self.board_service.create_column(NewBoardColumn::new(
                                        board_column_name,
                                        wip_limit.try_into().unwrap(),
                                        board_column_position.try_into().unwrap(),
                                    ));
                                }
                                return Ok(ControlFlow::Continue(()));
                            }
                        }
                    }
                }
            }
            PageView::ConfirmDialog => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => {
                            // go back
                            match &self.current_action_to_confirm {
                                ActionToConfirm::DeleteCard(page_view) => {
                                    self.page_view = page_view.clone();
                                }
                                ActionToConfirm::DeleteBoardColumn => {
                                    self.page_view = PageView::BoardDetails;
                                }
                                _ => {
                                    self.page_view = PageView::BoardDetails;
                                }
                            }
                        }
                        (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Left, KeyModifiers::NONE) => {
                            let total_options = self.confirm_dialog_state.0.len();
                            self.confirm_dialog_state.1.previous(total_options);
                        }
                        (KeyCode::Down, KeyModifiers::NONE)
                        | (KeyCode::Right, KeyModifiers::NONE) => {
                            let total_options = self.confirm_dialog_state.0.len();
                            self.confirm_dialog_state.1.next(total_options);
                        }
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            let options = &self.confirm_dialog_state.0;
                            let selected_index = self.confirm_dialog_state.1.selected_index;
                            let selected_option = &options[selected_index];

                            let confirmed = selected_option.data;
                            let action = self.current_action_to_confirm.clone();
                            debug!(
                                "About to proceed with selected action: {action:?}, confirmed: {confirmed}"
                            );
                            if selected_option.data {
                                // proceed with the action, right now the only action is delete of board column
                                if self.current_action_to_confirm
                                    == ActionToConfirm::DeleteBoardColumn
                                {
                                    // trigger the deletion of selected board column, fire-and-forget approach
                                    let board_column_id = &self.board_columns
                                        [self.current_column_index]
                                        .board_column
                                        .id;
                                    self.board_service.delete_column_by_id(board_column_id);
                                    self.page_view = PageView::BoardDetails;
                                    self.current_action_to_confirm = ActionToConfirm::NoAction;
                                } else if let ActionToConfirm::DeleteCard(page_view) =
                                    &self.current_action_to_confirm
                                {
                                    // trigger the removal of selected card, fire-and-forget approach
                                    let column_cards =
                                        &self.board_columns[self.current_column_index].cards;
                                    let card_list_state =
                                        &self.board_columns[self.current_column_index].list_state;
                                    if let Some(selected_card_index) = card_list_state.selected() {
                                        let card_to_delete = &column_cards[selected_card_index];
                                        self.card_service.delete_card_by_id(&card_to_delete.id);
                                    }
                                    self.page_view = page_view.clone();
                                    self.card_to_view = None;
                                    self.current_action_to_confirm = ActionToConfirm::NoAction;
                                }
                            } else {
                                // go back to board
                                self.page_view = PageView::BoardDetails;
                            }
                        }
                        _ => {}
                    }
                }
            }
            PageView::ManageCard { coming_from } => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => {
                            // go back to the previous page (coming_from)
                            // how to navigate from create vs. update/edit - comming from will contain the state
                            self.page_view = *coming_from.clone();
                        }
                        _ => {
                            // delegate the event to the underlying state
                            let result = self.manage_card_form_state.handle_event(event)?;
                            // based on the result decide if to create or update a card
                            if let ControlFlow::Break((
                                card_id,
                                column_id,
                                title,
                                description,
                                status,
                                blocked_reason,
                                created_at,
                                started_at,
                                completed_at,
                            )) = result
                            {
                                // process the collected data from the form
                                if let Some(card_id) = card_id {
                                    // in this case the id already exists, so it's update
                                    let status = if status == CardStatus::Active {
                                        "Active"
                                    } else {
                                        "Blocked"
                                    };
                                    self.card_service.update_card(Card {
                                        id: card_id,
                                        column_id,
                                        title,
                                        description: Some(description),
                                        status: status.to_string(),
                                        blocked_reason: Some(blocked_reason),
                                        created_at: created_at.unwrap_or_default(),
                                        started_at: started_at,
                                        completed_at: completed_at,
                                    });
                                } else {
                                    // in this case the id does not exist, so it's create
                                    let status = if status == CardStatus::Active {
                                        "Active"
                                    } else {
                                        "Blocked"
                                    };
                                    self.card_service.create_card(NewCard::new(
                                        column_id,
                                        title,
                                        Some(description),
                                        status.to_string(),
                                        Some(blocked_reason),
                                    ));
                                }
                                return Ok(ControlFlow::Continue(()));
                            }
                        }
                    }
                }
            }
            PageView::ViewCard => {
                if let Event::Key(key_event) = event {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Esc, KeyModifiers::NONE) => {
                            self.page_view = PageView::BoardDetails
                        }
                        (KeyCode::Up, KeyModifiers::NONE) => {
                            if self.view_card_scroll_offset > 0 {
                                self.view_card_scroll_offset -= 1;
                            }
                        }
                        (KeyCode::Down, KeyModifiers::NONE) => {
                            self.view_card_scroll_offset =
                                self.view_card_scroll_offset.saturating_add(1);
                        }
                        (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                            debug!("Handling removing of selected card from PageView::ViewCard");
                            let card_list_state =
                                self.board_columns[self.current_column_index].list_state;
                            if card_list_state.selected().is_some() {
                                self.current_action_to_confirm =
                                    ActionToConfirm::DeleteCard(ViewCard);
                                self.page_view = PageView::ConfirmDialog;
                            }
                        }
                        (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                            debug!("Handling the manage/edit card [update existing card]");
                            // enter edit mode just in case there is some card
                            if let Some(card) = self.card_to_view.clone() {
                                self.manage_card_form_state.clear(); // clear the form
                                // preset the form with existing data from selected card
                                self.manage_card_form_state.preset_with_card_data(card);
                                self.page_view = PageView::ManageCard {
                                    coming_from: Box::new(PageView::ViewCard),
                                };
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(std::ops::ControlFlow::Continue(()))
    }
}

/// `ManageBoardDetails` represents the statefull widget for a page responsible for:
/// - managing the cards of specific Kanban board and columns of Kanban Board
#[derive(Default)]
pub struct ManageBoardDetails {}

impl ManageBoardDetails {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for ManageBoardDetails {
    type State = ManageBoardDetailsState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match state.page_view {
            PageView::BoardDetails => {
                // get currently active board
                let active_board = {
                    let config = state.app_config.lock().unwrap();
                    config.current_board.clone()
                };

                let total_columns_num = state.board_columns.len();
                if total_columns_num == 0 {
                    let board_placeholder = Paragraph::new("No Board Columns Yet".to_string())
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(ratatui::style::Color::Gray))
                                .title(format!("Manage Board Details - {active_board}"))
                                .title_style(Style::default().bold()),
                        );
                    Widget::render(board_placeholder, area, buf);
                } else {
                    // prepare some styles
                    let normal_style = Style::default();
                    let highlighted_style =
                        Style::default().fg(ratatui::style::Color::Green).bold();

                    // figure out first the number of columns to display
                    let display_columns_num = {
                        let max_columns_display = 4;
                        if total_columns_num > max_columns_display {
                            max_columns_display
                        } else {
                            total_columns_num
                        }
                    };
                    // prepare the place(s) for rendering columns with equal width
                    let constraints = vec![Constraint::Fill(1); display_columns_num];
                    let column_areas = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(&constraints)
                        .split(area);

                    let window_start =
                        state.current_window_start(total_columns_num, display_columns_num);
                    let visible_columns =
                        &mut state.board_columns[window_start..window_start + display_columns_num];

                    // render the visible board columns evenly across the available area
                    for (index, column_area) in column_areas.iter().enumerate() {
                        let board_column_index = window_start + index;
                        let border_style = if board_column_index == state.current_column_index {
                            highlighted_style
                        } else {
                            normal_style
                        };
                        let board_column = &visible_columns[index].board_column;
                        let cards = &visible_columns[index].cards;
                        let cards_list_state = &mut visible_columns[index].list_state;

                        let board_column_name = board_column.name.clone();
                        let wip_limit = board_column.wip_limit;
                        let board_column_position = board_column.position;
                        let card_items: Vec<ListItem> = cards
                            .iter()
                            .map(|card| {
                                if state.marked_cards.contains_key(&card.id) {
                                    let card_title = card.title.to_string();
                                    ListItem::new(Text::from(Line::from(format!(
                                        "[X] {card_title}"
                                    ))))
                                } else {
                                    ListItem::new(Text::from(Line::from(card.title.to_string())))
                                }
                            })
                            .collect();
                        let column_cards_list = List::default().items(card_items).highlight_style(
                            Style::default().fg(Color::LightYellow).bg(Color::DarkGray),
                        );
                        let column_block_window = StatefulWindow::new(
                            &format!(
                                "{board_column_name} - (wip: {wip_limit}, position: {board_column_position})"
                            ),
                            column_cards_list,
                        ).title_style(Style::default().bold()).border_style(border_style);
                        StatefulWidget::render(
                            column_block_window,
                            *column_area,
                            buf,
                            cards_list_state,
                        );
                    }
                }
            }
            PageView::ManageBoardColumn => {
                // render the manage board column form
                let manage_board_column_form = ManageBoardColumnForm::new();
                let manage_board_column_form_state = &mut state.manage_board_column_form_state;
                StatefulWidget::render(
                    manage_board_column_form,
                    area,
                    buf,
                    manage_board_column_form_state,
                );
            }
            PageView::ConfirmDialog => {
                if state.current_action_to_confirm == ActionToConfirm::DeleteBoardColumn {
                    let selected_board =
                        &state.board_columns[state.current_column_index].board_column;
                    let selected_board_name = &selected_board.name;

                    let options = &state.confirm_dialog_state.0;
                    let confirm_dialog = ChoicePicker::new(options)
                        .layout(choice_picker::PickerLayout::VerticalList)
                        .block(
                            Block::default()
                                .title(format!("Do you want to delete selected board column -> {selected_board_name} ?"))
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Red)),
                        );
                    let confirm_dialog_state = &mut state.confirm_dialog_state.1;
                    StatefulWidget::render(
                        confirm_dialog,
                        Center::builder(area)
                            .horizontally(true)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                        confirm_dialog_state,
                    );
                } else if let ActionToConfirm::DeleteCard(_page_view) =
                    &state.current_action_to_confirm
                {
                    let card_list_state =
                        &state.board_columns[state.current_column_index].list_state;
                    if let Some(selected_index) = card_list_state.selected() {
                        let selected_card_title = &state.board_columns[state.current_column_index]
                            .cards[selected_index]
                            .title;

                        let options = &state.confirm_dialog_state.0;
                        let confirm_dialog = ChoicePicker::new(options)
                        .layout(choice_picker::PickerLayout::VerticalList)
                        .block(
                            Block::default()
                                .title(format!(
                                    "Do you want to delete selected card -> {selected_card_title} ?"
                                ))
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Red)),
                        );
                        let confirm_dialog_state = &mut state.confirm_dialog_state.1;
                        StatefulWidget::render(
                            confirm_dialog,
                            Center::builder(area)
                                .horizontally(true)
                                .vertically(false)
                                .build()
                                .center(),
                            buf,
                            confirm_dialog_state,
                        );
                    }
                }
            }
            PageView::ManageCard { coming_from: _ } => {
                // render the manage card form
                let manage_card_form = ManageCardForm::new();
                let manage_card_form_state = &mut state.manage_card_form_state;
                StatefulWidget::render(manage_card_form, area, buf, manage_card_form_state);
            }
            PageView::ViewCard => {
                if let Some(card) = &state.card_to_view {
                    let Card {
                        title,
                        status,
                        blocked_reason,
                        description,
                        created_at,
                        started_at,
                        completed_at,
                        id: _id,
                        column_id: _column_id,
                    } = card;

                    // prepare the layout and the places for rendering
                    let (
                        title_area,
                        status_created_at_area,
                        started_at_and_completed_at_area,
                        maybe_blocked_reason_area,
                        description_area,
                    ) = if let Some(blocked_reason) = blocked_reason
                        && !blocked_reason.is_empty()
                    {
                        let vertical_layout = Layout::vertical([
                            Constraint::Length(2), // place for title
                            Constraint::Length(1), // place for status + created_at
                            Constraint::Length(2), // place for started_at + completed_at
                            Constraint::Length(2), // place for blocked_reason
                            Constraint::Fill(1),   // place for description
                        ]);
                        let [
                            title_area,
                            status_created_at_area,
                            started_at_and_completed_at_area,
                            blocked_reason_area,
                            description_area,
                        ] = area.layout(&vertical_layout);
                        (
                            title_area,
                            status_created_at_area,
                            started_at_and_completed_at_area,
                            Some(blocked_reason_area),
                            description_area,
                        )
                    } else {
                        let vertical_layout = Layout::vertical([
                            Constraint::Length(2), // place for title
                            Constraint::Length(1), // place for status + created_at
                            Constraint::Length(2), // place for started_at + completed_at
                            Constraint::Fill(1),   // place for description
                        ]);
                        let [
                            title_area,
                            status_created_at_area,
                            started_at_and_completed_at_area,
                            description_area,
                        ] = area.layout(&vertical_layout);
                        (
                            title_area,
                            status_created_at_area,
                            started_at_and_completed_at_area,
                            None,
                            description_area,
                        )
                    };

                    let horizontal_row_layout =
                        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]);
                    let [status_area, created_at_area] =
                        status_created_at_area.layout(&horizontal_row_layout);
                    let [started_at_area, completed_at_area] =
                        started_at_and_completed_at_area.layout(&horizontal_row_layout);

                    // render title
                    let title_paragraph = Paragraph::new(title.to_string());
                    Widget::render(
                        title_paragraph,
                        Center::builder(title_area)
                            .horizontally(false)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );

                    // render the status
                    let status_style = if status == "Active" {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    };
                    let status_paragraph =
                        Paragraph::new(format!("status: {status}")).style(status_style);
                    Widget::render(
                        status_paragraph,
                        Center::builder(status_area)
                            .horizontally(false)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );

                    // render the created_at
                    let created_at_formatted = format_date_time(Some(*created_at));
                    let created_at_paragraph =
                        Paragraph::new(format!("created: {created_at_formatted}"));
                    Widget::render(
                        created_at_paragraph,
                        Center::builder(created_at_area)
                            .horizontally(false)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );

                    // render the started_at
                    let started_at_formatted = format_date_time(*started_at);
                    let started_at_paragraph =
                        Paragraph::new(format!("started: {started_at_formatted}"));
                    Widget::render(
                        started_at_paragraph,
                        Center::builder(started_at_area)
                            .horizontally(false)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );

                    // render the completed_at
                    let completed_at_formatted = format_date_time(*completed_at);
                    let completed_at_paragraph =
                        Paragraph::new(format!("completed: {completed_at_formatted}"));
                    Widget::render(
                        completed_at_paragraph,
                        Center::builder(completed_at_area)
                            .horizontally(false)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );

                    // render the optional blocked_reason
                    if let Some(blocked_reason) = blocked_reason
                        && let Some(blocked_reason_area) = maybe_blocked_reason_area
                    {
                        let blocked_reason_paragraph =
                            Paragraph::new(format!("reason: {blocked_reason}"))
                                .style(Style::default().fg(Color::Red));
                        Widget::render(
                            blocked_reason_paragraph,
                            Center::builder(blocked_reason_area)
                                .horizontally(false)
                                .vertically(false)
                                .build()
                                .center(),
                            buf,
                        );
                    }

                    // render the description markup as markdown with scroll support
                    if let Some(description) = description {
                        let description_markup_text =
                            the_other_tui_markdown::into_text(description);
                        let description_paragraph = Paragraph::new(description_markup_text)
                            .wrap(Wrap { trim: true })
                            .scroll((state.view_card_scroll_offset, 0));
                        Widget::render(
                            description_paragraph,
                            Center::builder(description_area)
                                .horizontally(false)
                                .vertically(false)
                                .build()
                                .center(),
                            buf,
                        );
                    }
                } else {
                    // this should not happen, but if yes, then let user know there is no card to display
                    let paragraph = Paragraph::new("No card to display");
                    Widget::render(
                        paragraph,
                        Center::builder(area)
                            .horizontally(true)
                            .vertically(false)
                            .build()
                            .center(),
                        buf,
                    );
                }
            }
        }
    }
}

fn format_date_time(maybe_date_time: Option<NaiveDateTime>) -> String {
    if let Some(date_time) = maybe_date_time {
        date_time.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        " - ".to_string()
    }
}
