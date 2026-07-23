use std::sync::Arc;

use tokio::{sync::mpsc::Sender, task::JoinHandle};
use tracing::{debug, error};

use crate::{
    model::board::{BoardColumn, BoardColumnRepo, NewBoardColumn},
    tui::event::events::{AppEvent, BoardEvent},
};

/// `BoardService` represents the async type of service for TUI needs
/// it encapsulates the `BoardColumnRepo` and acts as an asynchronous wrapper around it
/// sender `Sender<AppEvent>` is used as a asynchronous communication channel to get the
/// result of operation back to TUI
pub struct BoardService {
    board_column_repo: Arc<BoardColumnRepo>,
    sender: Sender<AppEvent>,
}

impl BoardService {
    pub fn new(board_column_repo: Arc<BoardColumnRepo>, sender: Sender<AppEvent>) -> Self {
        Self {
            board_column_repo,
            sender,
        }
    }

    pub fn create_column(&self, column: NewBoardColumn) -> JoinHandle<()> {
        let board_column_repo = self.board_column_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = board_column_repo.create_column(column).await;
            match result {
                Ok(created_board_column) => {
                    match sender
                        .send(AppEvent::Board(BoardEvent::BoardColumnCreated(
                            created_board_column,
                        )))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful creation of board column was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about creation of board column, error: {error}"
                            );
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Board(BoardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while creating board column was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of creating board column, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }

    pub fn get_column_by_id(&self, id: &str) -> JoinHandle<()> {
        let id = id.to_string();
        let board_column_repo = self.board_column_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = board_column_repo.get_column_by_id(&id).await;
            match result {
                Ok(fetched_board_column) => {
                    match sender
                        .send(AppEvent::Board(BoardEvent::BoardColumnFetched(
                            fetched_board_column,
                        )))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful fetching of board column was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about fetching of board column, error: {error}"
                            );
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Board(BoardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while fetching board column was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of fetching board column, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }

    pub fn list_columns(&self) -> JoinHandle<()> {
        let board_column_repo = self.board_column_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = board_column_repo.list_columns().await;
            match result {
                Ok(fetched_board_columns) => {
                    match sender
                        .send(AppEvent::Board(BoardEvent::BoardColumnsFetched(
                            fetched_board_columns,
                        )))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful fetching of board columns was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about fetching of board columns, error: {error}"
                            );
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Board(BoardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while fetching board columns was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of fetching board columns, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }

    pub fn update_column(&self, column: BoardColumn) -> JoinHandle<()> {
        let board_column_repo = self.board_column_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = board_column_repo.update_column(column).await;
            match result {
                Ok(updated_board_column) => {
                    match sender
                        .send(AppEvent::Board(BoardEvent::BoardColumnUpdated(
                            updated_board_column,
                        )))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful updating of board column was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about updating of board column, error: {error}"
                            );
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Board(BoardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while updating board column was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of updating board column, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }

    pub fn delete_column_by_id(&self, id: &str) -> JoinHandle<()> {
        let id = id.to_string();
        let board_column_repo = self.board_column_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = board_column_repo.delete_column_by_id(&id).await;
            match result {
                Ok(()) => {
                    match sender
                        .send(AppEvent::Board(BoardEvent::BoardColumnDeleted(
                            id.to_string(),
                        )))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful deleting of board column was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about deleting of board column, error: {error}"
                            );
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Board(BoardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while deleting board column was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of deleting board column, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{AppConfig, KanbanBoard};
    use crate::model::board::BoardColumnRepoError;
    use crate::model::connection::Connection;
    use crate::model::test_utils::{acquire_test_lock, reset_db_pool};
    use anyhow::Result;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    async fn init_repo(db_path: &Path) -> Result<crate::model::board::BoardColumnRepo> {
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
        Ok(crate::model::board::BoardColumnRepo::new(pool))
    }

    async fn receive_event(receiver: &mut mpsc::Receiver<AppEvent>) -> AppEvent {
        receiver
            .recv()
            .await
            .expect("expected one AppEvent from BoardService")
    }

    #[tokio::test]
    async fn create_column_sends_board_column_created_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_board_create.db");
        let repo = init_repo(&db_file).await?;
        let (sender, mut receiver) = mpsc::channel(1);
        let service = BoardService::new(Arc::new(repo), sender);

        let handle = service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        let event = receive_event(&mut receiver).await;
        handle.await.expect("BoardService task panicked");

        match event {
            AppEvent::Board(BoardEvent::BoardColumnCreated(column)) => {
                assert_eq!(column.name, "Todo");
                assert_eq!(column.wip_limit, 3);
                assert_eq!(column.position, 0);
                assert!(!column.id.is_empty());
            }
            other => panic!("expected BoardColumnCreated event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_column_by_id_sends_board_column_fetched_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_board_get.db");
        let repo = init_repo(&db_file).await?;
        let created = repo
            .create_column(NewBoardColumn::new(String::from("In Progress"), 2, 1))
            .await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = BoardService::new(Arc::new(repo), sender);

        let handle = service.get_column_by_id(&created.id);
        let event = receive_event(&mut receiver).await;
        handle.await.expect("BoardService task panicked");

        match event {
            AppEvent::Board(BoardEvent::BoardColumnFetched(column)) => {
                assert_eq!(column.id, created.id);
                assert_eq!(column.name, "In Progress");
                assert_eq!(column.wip_limit, 2);
                assert_eq!(column.position, 1);
            }
            other => panic!("expected BoardColumnFetched event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn list_columns_sends_board_columns_fetched_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_board_list.db");
        let repo = init_repo(&db_file).await?;

        repo.create_column(NewBoardColumn::new(String::from("Done"), 2, 2))
            .await?;
        repo.create_column(NewBoardColumn::new(String::from("Todo"), 5, 0))
            .await?;
        repo.create_column(NewBoardColumn::new(String::from("Doing"), 4, 1))
            .await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = BoardService::new(Arc::new(repo), sender);

        let handle = service.list_columns();
        let event = receive_event(&mut receiver).await;
        handle.await.expect("BoardService task panicked");

        match event {
            AppEvent::Board(BoardEvent::BoardColumnsFetched(columns)) => {
                assert_eq!(columns.len(), 3);
                assert_eq!(columns[0].position, 0);
                assert_eq!(columns[0].name, "Todo");
                assert_eq!(columns[1].position, 1);
                assert_eq!(columns[2].position, 2);
            }
            other => panic!("expected BoardColumnsFetched event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn update_column_sends_board_column_updated_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_board_update.db");
        let repo = init_repo(&db_file).await?;

        let created = repo
            .create_column(NewBoardColumn::new(String::from("Todo"), 3, 0))
            .await?;
        let updated_id = created.id.clone();
        let updated_column = BoardColumn {
            id: updated_id.clone(),
            name: String::from("Ready"),
            wip_limit: created.wip_limit,
            position: created.position,
            created_at: created.created_at,
        };

        let (sender, mut receiver) = mpsc::channel(1);
        let service = BoardService::new(Arc::new(repo), sender);

        let handle = service.update_column(updated_column);
        let event = receive_event(&mut receiver).await;
        handle.await.expect("BoardService task panicked");

        match event {
            AppEvent::Board(BoardEvent::BoardColumnUpdated(column)) => {
                assert_eq!(column.id, updated_id);
                assert_eq!(column.name, "Ready");
            }
            other => panic!("expected BoardColumnUpdated event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn delete_column_by_id_sends_board_column_deleted_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_board_delete.db");
        let repo = init_repo(&db_file).await?;

        let created = repo
            .create_column(NewBoardColumn::new(String::from("Todo"), 3, 0))
            .await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = BoardService::new(Arc::new(repo), sender);

        let handle = service.delete_column_by_id(&created.id).await;
        let event = receive_event(&mut receiver).await;
        handle.expect("BoardService task panicked");

        match event {
            AppEvent::Board(BoardEvent::BoardColumnDeleted(column_id)) => {
                assert_eq!(column_id, created.id);
            }
            other => panic!("expected BoardColumnDeleted event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_column_by_id_sends_error_event_when_not_found() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_board_get_not_found.db");
        let repo = init_repo(&db_file).await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = BoardService::new(Arc::new(repo), sender);

        let handle = service.get_column_by_id("unknown-id");
        let event = receive_event(&mut receiver).await;
        handle.await.expect("BoardService task panicked");

        match event {
            AppEvent::Board(BoardEvent::Error(BoardColumnRepoError::NotFound { id })) => {
                assert_eq!(id, "unknown-id");
            }
            other => panic!("expected BoardEvent::Error(NotFound), got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn create_column_handles_sender_failure_gracefully() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_board_sender_failed.db");
        let repo = init_repo(&db_file).await?;

        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);
        let service = BoardService::new(Arc::new(repo), sender);

        let handle = service.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0));
        handle.await.expect("BoardService task panicked");

        Ok(())
    }

    #[tokio::test]
    async fn delete_column_by_id_sends_error_event_when_not_found() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_board_delete_not_found.db");
        let repo = init_repo(&db_file).await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = BoardService::new(Arc::new(repo), sender);

        let handle = service.delete_column_by_id("missing-id").await;
        let event = receive_event(&mut receiver).await;
        handle.expect("BoardService task panicked");

        match event {
            AppEvent::Board(BoardEvent::Error(BoardColumnRepoError::DeleteFailed { id })) => {
                assert_eq!(id, "missing-id");
            }
            other => panic!("expected BoardEvent::Error(DeleteFailed), got {other:?}"),
        }

        Ok(())
    }
}
