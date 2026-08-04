#[cfg(test)]
mod tests {
    use crate::core::config::{AppConfig, KanbanBoard};
    use crate::model::board::{BoardColumn, BoardColumnRepo, BoardColumnRepoError, NewBoardColumn};
    use crate::model::card::{Card, CardRepo, NewCard};
    use crate::model::connection::Connection;
    use crate::model::test_utils::{acquire_test_lock, reset_db_pool};
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};
    use crate::tui::components::choice_picker::ChoicePicker;
    use crate::tui::components::notification_panel::NotificationMessage;
    use crate::tui::components::text_input::TextInput;
    use crate::tui::event::events::{AppEvent, BoardEvent, CardEvent};
    use crate::tui::page::common::EventHandler;
    use crate::tui::page::manage_board_details::PageView::{BoardDetails, ViewCard};
    use crate::tui::page::manage_board_details::confirm_dialog::ActionToConfirm;
    use crate::tui::page::manage_board_details::manage_board_column_form::{
        ManageBoardColumnFormField, ManageBoardColumnFormState,
    };
    use crate::tui::page::manage_board_details::manage_card_form::{
        CardStatus, ManageCardFormField, ManageCardFormPageView, ManageCardFormState,
    };
    use crate::tui::page::manage_board_details::{
        BoardColumnWithCards, ManageBoardDetails, ManageBoardDetailsState, PageView,
    };
    use crate::tui::service::board::BoardService;
    use crate::tui::service::card::CardService;
    use crate::tui::service::notifications::NotificationService;
    use anyhow::Result;
    use chrono::{DateTime, TimeZone, Utc};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::widgets::ListState;
    use ratatui::{buffer::Buffer, prelude::Rect, widgets::StatefulWidget};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::ops::ControlFlow;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    fn make_test_app_config(db_path: &Path) -> Arc<Mutex<AppConfig>> {
        let mut boards = HashMap::new();
        boards.insert(
            String::from("test-board"),
            KanbanBoard::new(
                String::from("Test Board"),
                String::from("A temporary board for tests"),
                PathBuf::from(format!("sqlite://{}", db_path.display())),
            ),
        );

        Arc::new(Mutex::new(AppConfig {
            current_board: String::from("test-board"),
            logging_config: Default::default(),
            kanban_boards: boards,
        }))
    }

    async fn init_repos(db_path: &Path) -> Result<(BoardColumnRepo, CardRepo)> {
        let _lock = acquire_test_lock();
        reset_db_pool();

        let mut boards = HashMap::new();
        boards.insert(
            String::from("test-board"),
            KanbanBoard::new(
                String::from("Test Board"),
                String::from("A temporary board for tests"),
                PathBuf::from(format!("sqlite://{}", db_path.display())),
            ),
        );

        let config = AppConfig {
            current_board: String::from("test-board"),
            logging_config: Default::default(),
            kanban_boards: boards,
        };

        let connection = Connection::new(
            Arc::new(Mutex::new(config)),
            db_path.parent().unwrap().to_path_buf(),
        );
        let pool = connection.get_db_connection_pool().await?;
        Ok((
            BoardColumnRepo::new(pool.clone()),
            CardRepo::new(pool.clone()),
        ))
    }

    async fn make_board_service(
        db_path: &Path,
    ) -> Result<(Arc<BoardService>, mpsc::Receiver<AppEvent>)> {
        let repo = init_repos(db_path).await?;
        let (sender, receiver) = mpsc::channel(10);
        let service = Arc::new(BoardService::new(Arc::new(repo.0), sender));
        Ok((service, receiver))
    }

    async fn make_card_service(
        db_path: &Path,
    ) -> Result<(Arc<CardService>, mpsc::Receiver<AppEvent>)> {
        let repo = init_repos(db_path).await?;
        let (sender, receiver) = mpsc::channel(10);
        let service = Arc::new(CardService::new(Arc::new(repo.1), sender));
        Ok((service, receiver))
    }

    fn make_notification_service() -> (
        Arc<NotificationService>,
        mpsc::Receiver<NotificationMessage>,
    ) {
        let (sender, receiver) = mpsc::channel(10);
        (Arc::new(NotificationService::new(sender)), receiver)
    }

    fn fill_text_input(input: &mut TextInput, text: &str) {
        for ch in text.chars() {
            let _ = input
                .handle_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(ch),
                    KeyModifiers::NONE,
                )))
                .unwrap();
        }
    }

    #[test]
    fn test_manage_board_column_form_state_new_initializes_defaults() {
        let state = ManageBoardColumnFormState::new();

        assert!(state.id.is_none());
        assert!(state.name.is_focused());
        assert_eq!(state.wip_limit.get_input_value(), 1);
        assert_eq!(state.position.get_input_value(), 0);
        assert!(!state.wip_limit.is_focused());
        assert!(!state.position.is_focused());
    }

    #[test]
    fn test_manage_board_column_form_state_preset_with_existing_data() {
        let mut state = ManageBoardColumnFormState::new();
        let board_column = BoardColumn {
            id: String::from("col-1"),
            name: String::from("In Progress"),
            wip_limit: 5,
            position: 2,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        };

        state.preset_with_board_column_data(board_column.clone());

        assert_eq!(state.id.as_deref(), Some("col-1"));
        assert_eq!(state.name.get_buffer(), board_column.name);
        assert_eq!(
            state.wip_limit.get_input_value(),
            board_column.wip_limit as i32
        );
        assert_eq!(
            state.position.get_input_value(),
            board_column.position as i32
        );
        assert!(state.name.is_focused());
        assert!(!state.wip_limit.is_focused());
        assert!(!state.position.is_focused());
    }

    #[test]
    fn test_manage_board_column_form_state_clear_resets_values_and_focus() {
        let mut state = ManageBoardColumnFormState::new();
        fill_text_input(&mut state.name, "Test Column");
        state.wip_limit.set_input_value(10).unwrap();
        state.position.set_input_value(3).unwrap();
        state.id = Some(String::from("existing-id"));
        state.current_field = ManageBoardColumnFormField::Position;

        state.clear();

        assert!(state.id.is_none());
        assert_eq!(state.name.get_buffer(), "");
        assert_eq!(state.wip_limit.get_input_value(), 1);
        assert_eq!(state.position.get_input_value(), 0);
        assert!(state.name.is_focused());
        assert!(!state.wip_limit.is_focused());
        assert!(!state.position.is_focused());
        assert!(matches!(
            state.current_field,
            ManageBoardColumnFormField::Name
        ));
    }

    #[test]
    fn test_manage_board_column_form_state_tab_moves_focus_through_fields() {
        let mut state = ManageBoardColumnFormState::new();

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(!state.name.is_focused());
        assert!(state.wip_limit.is_focused());

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(!state.wip_limit.is_focused());
        assert!(state.position.is_focused());

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(!state.position.is_focused());
        assert!(state.name.is_focused());
    }

    #[test]
    fn test_manage_board_column_form_state_submits_when_name_is_populated() {
        let mut state = ManageBoardColumnFormState::new();
        fill_text_input(&mut state.name, "In Progress");
        state.wip_limit.set_input_value(4).unwrap();
        state.position.set_input_value(1).unwrap();

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        match result {
            ControlFlow::Break((id, name, wip_limit, position)) => {
                assert!(id.is_none());
                assert_eq!(name, "In Progress");
                assert_eq!(wip_limit, 4);
                assert_eq!(position, 1);
            }
            _ => panic!("expected submission break"),
        }
    }

    #[test]
    fn test_manage_board_column_form_state_does_not_submit_when_name_is_empty() {
        let mut state = ManageBoardColumnFormState::new();
        state.wip_limit.set_input_value(4).unwrap();
        state.position.set_input_value(1).unwrap();

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(result, ControlFlow::Continue(())));
    }

    #[tokio::test]
    async fn test_manage_board_details_state_new_initializes_board_details_page() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_new.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);

        let state = ManageBoardDetailsState::new(
            app_config.clone(),
            board_service.clone(),
            card_service.clone(),
            notification_service.clone(),
        );

        let event = event_receiver
            .recv()
            .await
            .expect("expected board columns fetched event");
        assert!(matches!(
            event,
            AppEvent::Board(BoardEvent::BoardColumnsFetched(_))
        ));
        assert!(matches!(state.page_view, PageView::BoardDetails));
        assert!(state.board_columns.is_empty());
        assert_eq!(state.current_column_index, 0);
        assert!(matches!(
            state.current_action_to_confirm,
            ActionToConfirm::NoAction
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_updates_board_columns_and_clamps_index()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 10,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let fetched_columns = vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Todo"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }];

        state.handle_app_event(AppEvent::Board(BoardEvent::BoardColumnsFetched(
            fetched_columns.clone(),
        )));

        assert_eq!(
            state
                .board_columns
                .iter()
                .map(|board_column_tuple| board_column_tuple.board_column.clone())
                .collect::<Vec<BoardColumn>>(),
            fetched_columns
        );
        assert_eq!(state.current_column_index, 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_error_sends_notification()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_error.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, mut notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service,
            card_service,
            notification_service,
        );

        let _ = event_receiver.recv().await;

        state.handle_app_event(AppEvent::Board(BoardEvent::Error(
            BoardColumnRepoError::NotFound {
                id: String::from("boom"),
            },
        )));

        let notification = notification_receiver
            .recv()
            .await
            .expect("expected notification message");

        match notification {
            NotificationMessage::ErrorMsg(text, _) => {
                assert!(text.contains("boom"));
            }
            other => panic!("expected error notification, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_n_enters_manage_board_column()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: {
                let mut state = ManageBoardColumnFormState::new();
                fill_text_input(&mut state.name, "test");
                state
            },
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ManageBoardColumn));
        assert!(
            state
                .manage_board_column_form_state
                .name
                .get_buffer()
                .is_empty()
        );
        assert!(state.manage_board_column_form_state.name.is_focused());
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_e_presets_existing_column()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("In Progress"),
                    wip_limit: 2,
                    position: 1,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ManageBoardColumn));
        assert_eq!(
            state.manage_board_column_form_state.id.as_deref(),
            Some("column-1")
        );
        assert_eq!(
            state.manage_board_column_form_state.name.get_buffer(),
            "In Progress"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_d_enters_confirm_dialog()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("In Progress"),
                    wip_limit: 2,
                    position: 1,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ConfirmDialog));
        assert!(matches!(
            state.current_action_to_confirm,
            ActionToConfirm::DeleteBoardColumn
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_right_left_cycles_columns() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![
                BoardColumnWithCards::new(
                    BoardColumn {
                        id: String::from("column-1"),
                        name: String::from("Todo"),
                        wip_limit: 1,
                        position: 0,
                        created_at: Utc
                            .timestamp_opt(1_000_000, 0)
                            .single()
                            .unwrap()
                            .naive_utc(),
                    },
                    vec![],
                ),
                BoardColumnWithCards::new(
                    BoardColumn {
                        id: String::from("column-2"),
                        name: String::from("Doing"),
                        wip_limit: 2,
                        position: 1,
                        created_at: Utc
                            .timestamp_opt(1_000_000, 0)
                            .single()
                            .unwrap()
                            .naive_utc(),
                    },
                    vec![],
                ),
            ],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Right,
                KeyModifiers::NONE,
            )))
            .unwrap();
        assert_eq!(state.current_column_index, 1);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Right,
                KeyModifiers::NONE,
            )))
            .unwrap();
        assert_eq!(state.current_column_index, 0);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_column_index, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_confirm_dialog_yes_deletes_selected_column()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_delete.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service,
            notification_service,
        );
        let _ = event_receiver.recv().await;

        let create_handle =
            board_service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let created_event = event_receiver
            .recv()
            .await
            .expect("expected board column created event");
        create_handle.await.expect("BoardService task panicked");

        let created_column = match created_event {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => column,
            other => panic!("expected created event, got {other:?}"),
        };

        state.page_view = PageView::ConfirmDialog;
        state.current_action_to_confirm = ActionToConfirm::DeleteBoardColumn;
        state.board_columns = vec![BoardColumnWithCards::new(created_column.clone(), vec![])];
        state.current_column_index = 0;

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
            .unwrap();
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        let page_view = state.page_view.clone();
        println!("page_view: {page_view:?}");
        assert!(matches!(state.page_view, PageView::BoardDetails));
        let event = event_receiver
            .recv()
            .await
            .expect("expected board column deleted event");
        assert!(matches!(
            event,
            AppEvent::Board(BoardEvent::BoardColumnDeleted(_))
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_board_details_placeholder_when_no_columns()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_render_empty.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service,
            card_service,
            notification_service,
        );
        let _ = event_receiver.recv().await;

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf, &mut state);

        let rendered = buffer_to_string(&buf);
        let expected_output = "
┌Manage Board Details - test-board─────┐
│No Board Columns Yet                  │
│                                      │
│                                      │
└──────────────────────────────────────┘";
        assert_rendered_output(&rendered, expected_output);

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_manage_board_column_form_shows_form_fields()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageBoardColumn,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 50, 10);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
Create new column                                 
                                                  
┌Board Column Name:──────────────────────────────┐
│|                                               │
└────────────────────────────────────────────────┘
┌WorkInProgress Limit:───────────────────────────┐
└────────────────────────────────────────────────┘
┌Board Column Position:──────────────────────────┐
│ ▲ 0 ▼                                          │
└────────────────────────────────────────────────┘";

        assert_rendered_output(&rendered, expected_output);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_confirm_dialog_shows_title_and_options() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteBoardColumn,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 80, 6);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
          ┌Do you want to delete selected board column -> Todo ?─────┐          
          │  YES                                                     │          
          │> NO                                                      │          
          │                                                          │          
          │                                                          │          
          └──────────────────────────────────────────────────────────┘          ";

        assert_rendered_output(&rendered, expected_output);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_confirm_dialog_title_includes_column_name()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteBoardColumn,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 90, 6);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
               ┌Do you want to delete selected board column -> Todo ?─────┐               
               │  YES                                                     │               
               │> NO                                                      │               
               │                                                          │               
               │                                                          │               
               └──────────────────────────────────────────────────────────┘               ";

        assert_rendered_output(&rendered, expected_output);

        assert!(rendered.contains("Do you want to delete selected board column -> Todo ?",));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_view_card_scrolls_up_down() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: Some(Card {
                id: String::from("card-1"),
                column_id: String::from("column-1"),
                title: String::from("View Card Title"),
                description: Some(String::from(
                    "Line one line two line three line four line five line six line seven line eight line nine line ten",
                )),
                status: String::from("Active"),
                blocked_reason: None,
                created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
                started_at: None,
                completed_at: None,
            }),
            current_column_index: 0,
            page_view: PageView::ViewCard,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.view_card_scroll_offset, 1);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.view_card_scroll_offset, 0);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.view_card_scroll_offset, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_view_card_scrolls_description_in_output() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: Some(Card {
                id: String::from("card-1"),
                column_id: String::from("column-1"),
                title: String::from("View Card Title"),
                description: Some(String::from(
                    "Line one Line two Line three Line four Line five",
                )),
                status: String::from("Active"),
                blocked_reason: None,
                created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
                started_at: None,
                completed_at: None,
            }),
            current_column_index: 0,
            page_view: PageView::ViewCard,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let area = Rect::new(0, 0, 20, 6);
        let mut buf = Buffer::empty(area);
        let widget = ManageBoardDetails::new();

        widget.render(area, &mut buf, &mut state);
        let rendered0 = buffer_to_string(&buf);

        assert!(rendered0.contains("Line one"));
        assert!(rendered0.contains("Line two"));

        state.view_card_scroll_offset = 1;
        let mut buf = Buffer::empty(area);
        let widget = ManageBoardDetails::new();
        widget.render(area, &mut buf, &mut state);
        let rendered1 = buffer_to_string(&buf);

        assert!(rendered1.contains("Line three"));
        assert_ne!(rendered0, rendered1);

        Ok(())
    }

    #[test]
    fn test_manage_card_form_state_new_initializes_defaults() {
        let board_columns = vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }];

        let state = ManageCardFormState::new(board_columns.clone());

        assert!(state.id.is_none());
        assert_eq!(state.column_id.0.len(), 1);
        assert_eq!(state.column_id.0[0].label, "Backlog");
        assert_eq!(state.column_id.0[0].data, "column-1");
        assert_eq!(state.column_id.1.selected_index, 0);
        assert_eq!(state.title.get_buffer(), "");
        assert!(!state.title.is_focused());
        assert_eq!(state.description.get_text_as_string(), "");
        assert!(!state.description.is_focused());
        assert_eq!(state.status.1.selected_index, 0);
        assert_eq!(state.blocked_reason.get_buffer(), "");
        assert!(!state.blocked_reason.is_focused());
        assert!(matches!(state.current_field, ManageCardFormField::ColumnId));
    }

    #[test]
    fn test_manage_card_form_state_preset_with_card_data_sets_fields() {
        let board_columns = vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }];
        let mut state = ManageCardFormState::new(board_columns);
        let started_at = Utc
            .timestamp_opt(2_000_000, 0)
            .single()
            .unwrap()
            .naive_utc();
        let completed_at = Utc
            .timestamp_opt(3_000_000, 0)
            .single()
            .unwrap()
            .naive_utc();
        let card = Card {
            id: String::from("card-1"),
            column_id: String::from("column-1"),
            title: String::from("Fix bug"),
            description: Some(String::from("Line one\nLine two")),
            status: String::from("Blocked"),
            blocked_reason: Some(String::from("Waiting on API")),
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
            started_at: Some(started_at),
            completed_at: Some(completed_at),
        };

        state.preset_with_card_data(card);

        assert_eq!(state.id.as_deref(), Some("card-1"));
        assert_eq!(state.column_id.1.selected_index, 0);
        assert_eq!(state.title.get_buffer(), "Fix bug");
        assert_eq!(state.description.get_text_as_string(), "Line one\nLine two");
        assert_eq!(state.status.1.selected_index, 1);
        assert_eq!(state.blocked_reason.get_buffer(), "Waiting on API");
        assert_eq!(state.started_at, Some(started_at));
        assert_eq!(state.completed_at, Some(completed_at));
        assert!(matches!(state.current_field, ManageCardFormField::ColumnId));
    }

    #[test]
    fn test_manage_card_form_state_set_board_columns_resets_choices() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        state.set_board_columns(vec![BoardColumn {
            id: String::from("column-2"),
            name: String::from("In Progress"),
            wip_limit: 2,
            position: 1,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        assert_eq!(state.column_id.0.len(), 1);
        assert_eq!(state.column_id.0[0].label, "In Progress");
        assert_eq!(state.column_id.0[0].data, "column-2");
        assert_eq!(state.column_id.1.selected_index, 0);
    }

    #[test]
    fn test_manage_card_form_state_clear_resets_state() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);
        state.id = Some(String::from("card-1"));
        fill_text_input(&mut state.title, "Fix bug");
        state.description.set_text(vec![String::from("Line one")]);
        state.status.1.selected_index = 1;
        fill_text_input(&mut state.blocked_reason, "Blocked reason");
        state.started_at = Some(
            Utc.timestamp_opt(2_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        );
        state.completed_at = Some(
            Utc.timestamp_opt(3_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        );
        state.current_field = ManageCardFormField::BlockedReason;

        state.clear();

        assert!(state.id.is_none());
        assert_eq!(state.column_id.1.selected_index, 0);
        assert_eq!(state.title.get_buffer(), "");
        assert_eq!(state.description.get_text_as_string(), "");
        assert_eq!(state.status.1.selected_index, 0);
        assert_eq!(state.blocked_reason.get_buffer(), "");
        assert!(state.started_at.is_none());
        assert!(state.completed_at.is_none());
        assert!(matches!(state.current_field, ManageCardFormField::ColumnId));
    }

    #[test]
    fn test_manage_card_form_state_tab_moves_focus_through_fields() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(state.title.is_focused());
        assert!(matches!(state.current_field, ManageCardFormField::Title));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(state.description.is_focused());
        assert!(matches!(
            state.current_field,
            ManageCardFormField::Description
        ));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(matches!(state.current_field, ManageCardFormField::Status));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(matches!(
            state.current_field,
            ManageCardFormField::BlockedReason
        ));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(matches!(state.current_field, ManageCardFormField::ColumnId));
        assert!(!state.blocked_reason.is_focused());
    }

    #[test]
    fn test_manage_card_form_state_enter_in_description_does_not_submit() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);
        fill_text_input(&mut state.title, "Fix bug");

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert!(matches!(
            state.current_field,
            ManageCardFormField::Description
        ));

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(result, ControlFlow::Continue(())));
    }

    #[test]
    fn test_manage_card_form_state_ctrl_s_opens_started_at_picker_and_sets_date() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        let fixed_started_at = Utc
            .timestamp_opt(2_000_000, 0)
            .single()
            .unwrap()
            .naive_utc();

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        assert!(matches!(
            state.current_page,
            ManageCardFormPageView::DateTimePicker { .. }
        ));

        state
            .date_time_picker
            .preset_with_date_time(fixed_started_at);
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert_eq!(state.started_at, Some(fixed_started_at));
        assert!(matches!(state.current_page, ManageCardFormPageView::Form));
    }

    #[test]
    fn test_manage_card_form_state_ctrl_f_opens_completed_at_picker_and_sets_date() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        let fixed_completed_at = Utc
            .timestamp_opt(3_000_000, 0)
            .single()
            .unwrap()
            .naive_utc();

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('f'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        assert!(matches!(
            state.current_page,
            ManageCardFormPageView::DateTimePicker { .. }
        ));

        state
            .date_time_picker
            .preset_with_date_time(fixed_completed_at);
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert_eq!(state.completed_at, Some(fixed_completed_at));
        assert!(matches!(state.current_page, ManageCardFormPageView::Form));
    }

    #[test]
    fn test_manage_card_form_state_submit_includes_started_and_completed_at() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);

        fill_text_input(&mut state.title, "Fix bug");

        let fixed_started_at = Utc
            .timestamp_opt(2_000_000, 0)
            .single()
            .unwrap()
            .naive_utc();
        let fixed_completed_at = Utc
            .timestamp_opt(3_000_000, 0)
            .single()
            .unwrap()
            .naive_utc();

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        state
            .date_time_picker
            .preset_with_date_time(fixed_started_at);
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('f'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        state
            .date_time_picker
            .preset_with_date_time(fixed_completed_at);
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        match result {
            ControlFlow::Break((_, _, _, _, _, _, _, started_at, completed_at)) => {
                assert_eq!(started_at, Some(fixed_started_at));
                assert_eq!(completed_at, Some(fixed_completed_at));
            }
            _ => panic!("expected submission break"),
        }
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_k_enters_manage_card() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: {
                let mut state = ManageCardFormState::new(vec![BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Backlog"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                }]);
                fill_text_input(&mut state.title, "Should be cleared");
                state
            },
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('k'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(
            state.page_view,
            PageView::ManageCard { coming_from: _ }
        ));
        assert!(state.manage_card_form_state.title.get_buffer().is_empty());
        assert!(matches!(
            state.manage_card_form_state.current_field,
            ManageCardFormField::ColumnId
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_confirm_dialog_yes_deletes_selected_card() -> Result<()>
    {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_delete_card.db");
        let (board_service, mut board_event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut card_event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service.clone(),
            notification_service,
        );
        let _ = board_event_receiver.recv().await;

        let create_column_handle =
            board_service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let created_column = match board_event_receiver
            .recv()
            .await
            .expect("expected board column created event")
        {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => column,
            other => panic!("expected created event, got {other:?}"),
        };
        create_column_handle
            .await
            .expect("BoardService task panicked");

        let create_card_handle = card_service.create_card(NewCard::new(
            created_column.id.clone(),
            String::from("Card 1"),
            Some(String::from("Description")),
            String::from("Active"),
            None,
        ));
        let created_card = match card_event_receiver
            .recv()
            .await
            .expect("expected card created event")
        {
            AppEvent::Card(CardEvent::CardCreated(card)) => card,
            other => panic!("expected created event, got {other:?}"),
        };
        create_card_handle.await.expect("CardService task panicked");

        let mut list_state = ListState::default();
        list_state.select(Some(0));
        state.page_view = PageView::ConfirmDialog;
        state.current_action_to_confirm = ActionToConfirm::DeleteCard(BoardDetails);
        state.board_columns = vec![BoardColumnWithCards::new(
            created_column.clone(),
            vec![created_card.clone()],
        )];
        state.board_columns[0].list_state = list_state;
        state.current_column_index = 0;

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
            .unwrap();
        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::BoardDetails));
        let event = card_event_receiver
            .recv()
            .await
            .expect("expected card deleted event");
        assert!(matches!(event, AppEvent::Card(CardEvent::CardDeleted(_))));

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_ctrl_r_in_view_card_enters_confirm_dialog()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir
            .path()
            .join("manage_board_details_view_card_confirm.db");
        let (board_service, mut board_event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut card_event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service.clone(),
            notification_service,
        );
        let _ = board_event_receiver.recv().await;

        let create_column_handle =
            board_service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let created_column = match board_event_receiver
            .recv()
            .await
            .expect("expected board column created event")
        {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => column,
            other => panic!("expected created event, got {other:?}"),
        };
        create_column_handle
            .await
            .expect("BoardService task panicked");

        let create_card_handle = card_service.create_card(NewCard::new(
            created_column.id.clone(),
            String::from("Card 1"),
            Some(String::from("Description")),
            String::from("Active"),
            None,
        ));
        let created_card = match card_event_receiver
            .recv()
            .await
            .expect("expected card created event")
        {
            AppEvent::Card(CardEvent::CardCreated(card)) => card,
            other => panic!("expected created event, got {other:?}"),
        };
        create_card_handle.await.expect("CardService task panicked");

        let mut list_state = ListState::default();
        list_state.select(Some(0));
        state.page_view = PageView::ViewCard;
        state.card_to_view = Some(created_card.clone());
        state.board_columns = vec![BoardColumnWithCards::new(
            created_column.clone(),
            vec![created_card.clone()],
        )];
        state.board_columns[0].list_state = list_state;
        state.current_column_index = 0;

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('r'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ConfirmDialog));
        assert_eq!(
            state.current_action_to_confirm,
            ActionToConfirm::DeleteCard(ViewCard)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_confirm_dialog_yes_deletes_selected_card_from_view_card()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir
            .path()
            .join("manage_board_details_delete_card_view_card.db");
        let (board_service, mut board_event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut card_event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service.clone(),
            notification_service,
        );
        let _ = board_event_receiver.recv().await;

        let create_column_handle =
            board_service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let created_column = match board_event_receiver
            .recv()
            .await
            .expect("expected board column created event")
        {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => column,
            other => panic!("expected created event, got {other:?}"),
        };
        create_column_handle
            .await
            .expect("BoardService task panicked");

        let create_card_handle = card_service.create_card(NewCard::new(
            created_column.id.clone(),
            String::from("Card 1"),
            Some(String::from("Description")),
            String::from("Active"),
            None,
        ));
        let created_card = match card_event_receiver
            .recv()
            .await
            .expect("expected card created event")
        {
            AppEvent::Card(CardEvent::CardCreated(card)) => card,
            other => panic!("expected created event, got {other:?}"),
        };
        create_card_handle.await.expect("CardService task panicked");

        let mut list_state = ListState::default();
        list_state.select(Some(0));
        state.page_view = PageView::ConfirmDialog;
        state.current_action_to_confirm = ActionToConfirm::DeleteCard(ViewCard);
        state.card_to_view = Some(created_card.clone());
        state.board_columns = vec![BoardColumnWithCards::new(
            created_column.clone(),
            vec![created_card.clone()],
        )];
        state.board_columns[0].list_state = list_state;
        state.current_column_index = 0;
        state.confirm_dialog_state.1.selected_index = 0;

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::ViewCard));
        assert!(state.card_to_view.is_none());
        assert_eq!(state.current_action_to_confirm, ActionToConfirm::NoAction);

        let event = card_event_receiver
            .recv()
            .await
            .expect("expected card deleted event");
        assert!(matches!(event, AppEvent::Card(CardEvent::CardDeleted(_))));

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_manage_card_form_shows_form_title() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageCard {
                coming_from: Box::new(PageView::BoardDetails),
            },
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![BoardColumn {
                id: String::from("column-1"),
                name: String::from("Todo"),
                wip_limit: 3,
                position: 0,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
            }]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 60, 24);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
Create new card                                             
                                                            
┌Column Id:────────────────────────────────────────────────┐
│                         < Todo >                         │
└──────────────────────────────────────────────────────────┘
┌Card Title:───────────────────────────────────────────────┐
│|                                                         │
└──────────────────────────────────────────────────────────┘
┌Card Description:─────────────────────────────────────────┐
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
└──────────────────────────────────────────────────────────┘
┌Status:───────────────────────────────────────────────────┐
│                        < Active >                        │
└──────────────────────────────────────────────────────────┘
┌Blocked Reason:───────────────────────────────────────────┐
│|                                                         │
└──────────────────────────────────────────────────────────┘";

        assert_rendered_output(&rendered, expected_output);

        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_manage_card_form_exact_output() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageCard {
                coming_from: Box::new(PageView::BoardDetails),
            },
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![BoardColumn {
                id: String::from("column-1"),
                name: String::from("Todo"),
                wip_limit: 3,
                position: 0,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
            }]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 60, 24);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
Create new card                                             
                                                            
┌Column Id:────────────────────────────────────────────────┐
│                         < Todo >                         │
└──────────────────────────────────────────────────────────┘
┌Card Title:───────────────────────────────────────────────┐
│|                                                         │
└──────────────────────────────────────────────────────────┘
┌Card Description:─────────────────────────────────────────┐
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
└──────────────────────────────────────────────────────────┘
┌Status:───────────────────────────────────────────────────┐
│                        < Active >                        │
└──────────────────────────────────────────────────────────┘
┌Blocked Reason:───────────────────────────────────────────┐
│|                                                         │
└──────────────────────────────────────────────────────────┘";

        assert_rendered_output(&rendered, expected_output);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_render_confirm_dialog_for_card_delete() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![Card {
                    id: String::from("card-1"),
                    column_id: String::from("column-1"),
                    title: String::from("Fix bug"),
                    description: Some(String::from("Desc")),
                    status: String::from("Active"),
                    blocked_reason: None,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                    started_at: None,
                    completed_at: None,
                }],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteCard(BoardDetails),
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };
        state.board_columns[0].list_state.select(Some(0));

        let widget = ManageBoardDetails::new();
        let area = Rect::new(0, 0, 80, 6);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
          ┌Do you want to delete selected card -> Fix bug ?──────────┐          
          │  YES                                                     │          
          │> NO                                                      │          
          │                                                          │          
          │                                                          │          
          └──────────────────────────────────────────────────────────┘          ";

        assert_rendered_output(&rendered, expected_output);
        Ok(())
    }

    #[test]
    fn test_manage_card_form_state_submits_when_title_populated_from_column_id() {
        let mut state = ManageCardFormState::new(vec![BoardColumn {
            id: String::from("column-1"),
            name: String::from("Backlog"),
            wip_limit: 3,
            position: 0,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
        }]);
        fill_text_input(&mut state.title, "Fix bug");

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        match result {
            ControlFlow::Break((
                id,
                column_id,
                title,
                description,
                status,
                blocked_reason,
                created_at,
                started_at,
                completed_at,
            )) => {
                assert!(id.is_none());
                assert_eq!(column_id, "column-1");
                assert_eq!(title, "Fix bug");
                assert_eq!(description, "");
                assert_eq!(status, CardStatus::Active);
                assert_eq!(blocked_reason, "");
                assert!(created_at.is_none(), "expecting None for 'created_at'");
                assert!(started_at.is_none(), "expecting None for 'started_at'");
                assert!(completed_at.is_none(), "expecting None for 'completed_at'");
            }
            _ => panic!("expected submission break"),
        }
    }

    #[tokio::test]
    async fn test_manage_board_details_state_confirm_dialog_no_returns_to_board_details()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ConfirmDialog,
            current_action_to_confirm: ActionToConfirm::DeleteBoardColumn,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(state.page_view, PageView::BoardDetails));
        assert!(matches!(
            state.current_action_to_confirm,
            ActionToConfirm::DeleteBoardColumn
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_down_moves_card_selection() -> Result<()>
    {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![
                    Card {
                        id: String::from("card-1"),
                        column_id: String::from("column-1"),
                        title: String::from("First"),
                        description: Some(String::from("Desc")),
                        status: String::from("Active"),
                        blocked_reason: None,
                        created_at: Utc
                            .timestamp_opt(1_000_000, 0)
                            .single()
                            .unwrap()
                            .naive_utc(),
                        started_at: None,
                        completed_at: None,
                    },
                    Card {
                        id: String::from("card-2"),
                        column_id: String::from("column-1"),
                        title: String::from("Second"),
                        description: Some(String::from("Desc")),
                        status: String::from("Active"),
                        blocked_reason: None,
                        created_at: Utc
                            .timestamp_opt(1_000_000, 0)
                            .single()
                            .unwrap()
                            .naive_utc(),
                        started_at: None,
                        completed_at: None,
                    },
                ],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        assert!(state.board_columns[0].list_state.selected().is_none());

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();

        assert_eq!(state.board_columns[0].list_state.selected(), Some(0));

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();

        assert_eq!(state.board_columns[0].list_state.selected(), Some(1));
        Ok(())
    }

    #[tokio::test]
    async fn test_current_window_start_returns_zero_when_total_columns_fit() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
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
        };

        assert_eq!(state.current_window_start(2, 4), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_current_window_start_clamps_window_at_end() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 4,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        assert_eq!(state.current_window_start(5, 4), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_cards_fetched_appends_cards()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let second_pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(second_pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let cards = vec![
            Card {
                id: String::from("card-1"),
                column_id: String::from("column-1"),
                title: String::from("First Card"),
                description: Some(String::from("Desc")),
                status: String::from("Active"),
                blocked_reason: None,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
                started_at: None,
                completed_at: None,
            },
            Card {
                id: String::from("card-2"),
                column_id: String::from("column-1"),
                title: String::from("Second Card"),
                description: Some(String::from("Desc")),
                status: String::from("Active"),
                blocked_reason: None,
                created_at: Utc
                    .timestamp_opt(1_000_000, 0)
                    .single()
                    .unwrap()
                    .naive_utc(),
                started_at: None,
                completed_at: None,
            },
        ];

        state.handle_app_event(AppEvent::Card(CardEvent::CardsFetched(cards.clone())));

        assert_eq!(state.board_columns[0].cards.len(), 2);
        assert_eq!(state.board_columns[0].cards[0].title, "First Card");
        assert_eq!(state.board_columns[0].cards[1].title, "Second Card");
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_app_event_card_updated_replaces_card()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let second_pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let original_card = Card {
            id: String::from("card-1"),
            column_id: String::from("column-1"),
            title: String::from("Original Title"),
            description: Some(String::from("Desc")),
            status: String::from("Active"),
            blocked_reason: None,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
            started_at: None,
            completed_at: None,
        };
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(second_pool.clone()))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![BoardColumnWithCards::new(
                BoardColumn {
                    id: String::from("column-1"),
                    name: String::from("Todo"),
                    wip_limit: 3,
                    position: 0,
                    created_at: Utc
                        .timestamp_opt(1_000_000, 0)
                        .single()
                        .unwrap()
                        .naive_utc(),
                },
                vec![original_card.clone()],
            )],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::BoardDetails,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let updated_card = Card {
            id: String::from("card-1"),
            column_id: String::from("column-1"),
            title: String::from("Updated Title"),
            description: Some(String::from("Desc")),
            status: String::from("Active"),
            blocked_reason: None,
            created_at: Utc
                .timestamp_opt(1_000_000, 0)
                .single()
                .unwrap()
                .naive_utc(),
            started_at: None,
            completed_at: None,
        };

        state.handle_app_event(AppEvent::Card(CardEvent::CardUpdated(updated_card)));

        assert_eq!(state.board_columns[0].cards.len(), 1);
        assert_eq!(state.board_columns[0].cards[0].title, "Updated Title");
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_manage_board_column_enter_creates_column()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir
            .path()
            .join("manage_board_details_create_column.db");
        let (board_service, mut event_receiver) = make_board_service(&db_file).await?;
        let (card_service, _event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service,
            notification_service,
        );
        let _ = event_receiver.recv().await;

        state.page_view = PageView::ManageBoardColumn;
        state.manage_board_column_form_state.clear();
        fill_text_input(&mut state.manage_board_column_form_state.name, "New Column");
        state
            .manage_board_column_form_state
            .wip_limit
            .set_input_value(2)
            .unwrap();
        state
            .manage_board_column_form_state
            .position
            .set_input_value(1)
            .unwrap();

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        let event = event_receiver
            .recv()
            .await
            .expect("expected board column created event");
        assert!(matches!(
            event,
            AppEvent::Board(BoardEvent::BoardColumnCreated(_))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_manage_card_enter_creates_card()
    -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("manage_board_details_create_card.db");
        let (board_service, mut board_event_receiver) = make_board_service(&db_file).await?;
        let (card_service, mut card_event_receiver) = make_card_service(&db_file).await?;
        let (notification_service, _notification_receiver) = make_notification_service();
        let app_config = make_test_app_config(&db_file);
        let mut state = ManageBoardDetailsState::new(
            app_config,
            board_service.clone(),
            card_service.clone(),
            notification_service,
        );
        let _ = board_event_receiver.recv().await;

        let create_column_handle =
            board_service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let created_event = board_event_receiver
            .recv()
            .await
            .expect("expected board column created event");
        create_column_handle
            .await
            .expect("BoardService task panicked");

        let created_column = match created_event {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => column,
            other => panic!("expected created event, got {other:?}"),
        };

        state.page_view = PageView::ManageCard {
            coming_from: Box::new(PageView::BoardDetails),
        };
        state.manage_card_form_state = ManageCardFormState::new(vec![created_column.clone()]);
        fill_text_input(&mut state.manage_card_form_state.title, "New Card");

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        let event = card_event_receiver
            .recv()
            .await
            .expect("expected card created event");
        assert!(matches!(event, AppEvent::Card(CardEvent::CardCreated(_))));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_esc_from_manage_board_column_returns_board_details()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageBoardColumn,
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();

        assert!(matches!(state.page_view, PageView::BoardDetails));
        Ok(())
    }

    #[tokio::test]
    async fn test_manage_board_details_state_handle_event_esc_from_manage_card_returns_board_details()
    -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let board_column_repo = Arc::new(BoardColumnRepo::new(Arc::new(pool.clone())));
        let mut state = ManageBoardDetailsState {
            app_config: Arc::new(Mutex::new(AppConfig::default())),
            board_service: Arc::new(BoardService::new(
                board_column_repo.clone(),
                tokio::sync::mpsc::channel(1).0,
            )),
            card_service: Arc::new(CardService::new(
                Arc::new(CardRepo::new(Arc::new(pool))),
                tokio::sync::mpsc::channel(1).0,
            )),
            notification_service: Arc::new(NotificationService::new(
                tokio::sync::mpsc::channel(1).0,
            )),
            board_columns: vec![],
            card_to_view: None,
            current_column_index: 0,
            page_view: PageView::ManageCard {
                coming_from: Box::new(PageView::BoardDetails),
            },
            current_action_to_confirm: ActionToConfirm::NoAction,
            view_card_scroll_offset: 0,
            manage_board_column_form_state: ManageBoardColumnFormState::new(),
            manage_card_form_state: ManageCardFormState::new(vec![]),
            confirm_dialog_state: ChoicePicker::confirmation_dialog(),
            marked_cards: HashMap::new(),
        };

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();

        assert!(matches!(state.page_view, PageView::BoardDetails));
        Ok(())
    }
}
