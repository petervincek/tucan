use std::sync::Arc;

use tokio::{sync::mpsc::Sender, task::JoinHandle};
use tracing::{debug, error};

use crate::{
    model::card::{Card, CardRepo, NewCard},
    tui::event::events::{AppEvent, CardEvent},
};

/// `CardService` represents the async type of service for TUI needs
/// it encapsulates the `CardColumnRepo` and acts as an asynchronous wrapper around it
/// sender `Sender<AppEvent>` is used as a asynchronous communication channel to get the
/// result of operation back to TUI
pub struct CardService {
    card_repo: Arc<CardRepo>,
    sender: Sender<AppEvent>,
}

impl CardService {
    pub fn new(card_repo: Arc<CardRepo>, sender: Sender<AppEvent>) -> Self {
        Self { card_repo, sender }
    }

    pub fn create_card(&self, card: NewCard) -> JoinHandle<()> {
        let card_repo = self.card_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = card_repo.create_card(card).await;
            match result {
                Ok(created_card) => {
                    match sender
                        .send(AppEvent::Card(CardEvent::CardCreated(created_card)))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful creation of card was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!("Failure sending event about creation of card, error: {error}");
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Card(CardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while creating card was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of creating card, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }

    pub fn get_card_by_id(&self, id: &str) -> JoinHandle<()> {
        let id = id.to_string();
        let card_repo = self.card_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = card_repo.get_card_by_id(&id).await;
            match result {
                Ok(fetched_card) => {
                    match sender
                        .send(AppEvent::Card(CardEvent::CardFetched(fetched_card)))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful fetching of card was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!("Failure sending event about fetching of card, error: {error}");
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Card(CardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while fetching card was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of fetching card, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }

    pub fn list_columns(&self, column_id: &str) -> JoinHandle<()> {
        let column_id = column_id.to_string();
        let card_repo = self.card_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = card_repo.list_cards_for_column(&column_id).await;
            match result {
                Ok(fetched_cards) => {
                    match sender
                        .send(AppEvent::Card(CardEvent::CardsFetched(fetched_cards)))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful fetching of cards was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!("Failure sending event about fetching of cards, error: {error}");
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Card(CardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while fetching cards was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of fetching cards, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }

    pub fn update_card(&self, card: Card) -> JoinHandle<()> {
        let card_repo = self.card_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = card_repo.update_card(card).await;
            match result {
                Ok(updated_card) => {
                    match sender
                        .send(AppEvent::Card(CardEvent::CardUpdated(updated_card)))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful updating of card was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!("Failure sending event about updating of card, error: {error}");
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Card(CardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while updating card was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of updating card, error: {error}"
                            );
                        }
                    };
                }
            }
        })
    }

    pub fn delete_card_by_id(&self, id: &str) -> JoinHandle<()> {
        let id = id.to_string();
        let card_repo = self.card_repo.clone();
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = card_repo.delete_card_by_id(&id).await;
            match result {
                Ok(()) => {
                    match sender
                        .send(AppEvent::Card(CardEvent::CardDeleted(id.to_string())))
                        .await
                    {
                        Ok(_) => {
                            debug!(
                                "Event signaling successful deleting of card was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!("Failure sending event about deleting of card, error: {error}");
                        }
                    }
                }
                Err(error) => {
                    match sender.send(AppEvent::Card(CardEvent::Error(error))).await {
                        Ok(_) => {
                            debug!(
                                "Event signalling failure while deleting card was sent through event bus"
                            );
                        }
                        Err(error) => {
                            error!(
                                "Failure sending event about failure of deleting card, error: {error}"
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
    use crate::model::board::{BoardColumnRepo, NewBoardColumn};
    use crate::model::card::CardRepoError;
    use crate::model::connection::Connection;
    use crate::model::test_utils::{acquire_test_lock, reset_db_pool};
    use anyhow::Result;
    use sqlx::{Pool, Sqlite};
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    async fn init_repo(db_path: &Path) -> Result<(CardRepo, Arc<Pool<Sqlite>>)> {
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
        let board_column_repo = Arc::new(BoardColumnRepo::new(pool.clone()));
        Ok((CardRepo::new(pool.clone(), board_column_repo), pool))
    }

    async fn receive_event(receiver: &mut mpsc::Receiver<AppEvent>) -> AppEvent {
        receiver
            .recv()
            .await
            .expect("expected one AppEvent from CardService")
    }

    async fn insert_test_column(pool: Arc<Pool<Sqlite>>, name: &str) -> Result<String> {
        let column_repo = BoardColumnRepo::new(pool.clone());
        let created = column_repo
            .create_column(NewBoardColumn::new(String::from(name), 5, 0))
            .await?;
        Ok(created.id)
    }

    #[tokio::test]
    async fn create_card_sends_card_created_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_card_create.db");
        let (repo, pool) = init_repo(&db_file).await?;
        let column_id = insert_test_column(pool.clone(), "Todo").await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = CardService::new(Arc::new(repo), sender);

        let handle = service.create_card(NewCard::new(
            column_id.clone(),
            String::from("Test card"),
            Some(String::from("Description")),
            String::from("Active"),
            None,
        ));

        let event = receive_event(&mut receiver).await;
        handle.await.expect("CardService task panicked");

        match event {
            AppEvent::Card(CardEvent::CardCreated(card)) => {
                assert_eq!(card.column_id, column_id);
                assert_eq!(card.title, "Test card");
                assert_eq!(card.status, "Active");
                assert!(!card.id.is_empty());
            }
            other => panic!("expected CardCreated event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_card_by_id_sends_card_fetched_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_card_get.db");
        let (repo, pool) = init_repo(&db_file).await?;
        let column_id = insert_test_column(pool.clone(), "Backlog").await?;

        let created = repo
            .create_card(NewCard::new(
                column_id.clone(),
                String::from("Fetch me"),
                None,
                String::from("Active"),
                None,
            ))
            .await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = CardService::new(Arc::new(repo), sender);

        let handle = service.get_card_by_id(&created.id);
        let event = receive_event(&mut receiver).await;
        handle.await.expect("CardService task panicked");

        match event {
            AppEvent::Card(CardEvent::CardFetched(card)) => {
                assert_eq!(card.id, created.id);
                assert_eq!(card.title, "Fetch me");
                assert_eq!(card.column_id, column_id);
            }
            other => panic!("expected CardFetched event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn list_columns_sends_cards_fetched_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_card_list.db");
        let (repo, pool) = init_repo(&db_file).await?;
        let column_id = insert_test_column(pool.clone(), "In Progress").await?;

        repo.create_card(NewCard::new(
            column_id.clone(),
            String::from("B first"),
            None,
            String::from("Active"),
            None,
        ))
        .await?;
        repo.create_card(NewCard::new(
            column_id.clone(),
            String::from("A second"),
            None,
            String::from("Blocked"),
            Some(String::from("Waiting")),
        ))
        .await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = CardService::new(Arc::new(repo), sender);

        let handle = service.list_columns(&column_id);
        let event = receive_event(&mut receiver).await;
        handle.await.expect("CardService task panicked");

        match event {
            AppEvent::Card(CardEvent::CardsFetched(cards)) => {
                assert_eq!(cards.len(), 2);
                assert_eq!(cards[0].title, "A second");
                assert_eq!(cards[1].title, "B first");
            }
            other => panic!("expected CardsFetched event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn update_card_sends_card_updated_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_card_update.db");
        let (repo, pool) = init_repo(&db_file).await?;
        let column_id = insert_test_column(pool.clone(), "Review").await?;

        let created = repo
            .create_card(NewCard::new(
                column_id.clone(),
                String::from("Update me"),
                None,
                String::from("Active"),
                None,
            ))
            .await?;

        let updated_card = Card {
            id: created.id.clone(),
            column_id: created.column_id.clone(),
            title: String::from("Updated title"),
            description: created.description.clone(),
            status: String::from("Blocked"),
            blocked_reason: Some(String::from("Waiting")),
            created_at: created.created_at,
            started_at: created.started_at,
            completed_at: created.completed_at,
        };

        let (sender, mut receiver) = mpsc::channel(1);
        let service = CardService::new(Arc::new(repo), sender);

        let handle = service.update_card(updated_card);
        let event = receive_event(&mut receiver).await;
        handle.await.expect("CardService task panicked");

        match event {
            AppEvent::Card(CardEvent::CardUpdated(card)) => {
                assert_eq!(card.id, created.id);
                assert_eq!(card.title, "Updated title");
                assert_eq!(card.status, "Blocked");
                assert_eq!(card.blocked_reason.as_deref(), Some("Waiting"));
            }
            other => panic!("expected CardUpdated event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn delete_card_by_id_sends_card_deleted_event() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_card_delete.db");
        let (repo, pool) = init_repo(&db_file).await?;
        let column_id = insert_test_column(pool.clone(), "Done").await?;

        let created = repo
            .create_card(NewCard::new(
                column_id,
                String::from("Delete me"),
                None,
                String::from("Active"),
                None,
            ))
            .await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = CardService::new(Arc::new(repo), sender);

        let handle = service.delete_card_by_id(&created.id);
        let event = receive_event(&mut receiver).await;
        handle.await.expect("CardService task panicked");

        match event {
            AppEvent::Card(CardEvent::CardDeleted(card_id)) => {
                assert_eq!(card_id, created.id);
            }
            other => panic!("expected CardDeleted event, got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_card_by_id_sends_error_event_when_not_found() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_card_get_not_found.db");
        let (repo, _) = init_repo(&db_file).await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = CardService::new(Arc::new(repo), sender);

        let handle = service.get_card_by_id("unknown-id");
        let event = receive_event(&mut receiver).await;
        handle.await.expect("CardService task panicked");

        match event {
            AppEvent::Card(CardEvent::Error(CardRepoError::NotFound { id })) => {
                assert_eq!(id, "unknown-id");
            }
            other => panic!("expected CardEvent::Error(NotFound), got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn delete_card_by_id_sends_error_event_when_not_found() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_card_delete_not_found.db");
        let (repo, _) = init_repo(&db_file).await?;

        let (sender, mut receiver) = mpsc::channel(1);
        let service = CardService::new(Arc::new(repo), sender);

        let handle = service.delete_card_by_id("missing-id");
        let event = receive_event(&mut receiver).await;
        handle.await.expect("CardService task panicked");

        match event {
            AppEvent::Card(CardEvent::Error(CardRepoError::DeleteFailed { id })) => {
                assert_eq!(id, "missing-id");
            }
            other => panic!("expected CardEvent::Error(DeleteFailed), got {other:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn create_card_handles_sender_failure_gracefully() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("service_card_sender_failed.db");
        let (repo, pool) = init_repo(&db_file).await?;
        let column_id = insert_test_column(pool.clone(), "Dropped").await?;

        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);
        let service = CardService::new(Arc::new(repo), sender);

        let handle = service.create_card(NewCard::new(
            column_id,
            String::from("Silent card"),
            None,
            String::from("Active"),
            None,
        ));

        handle.await.expect("CardService task panicked");
        Ok(())
    }
}
