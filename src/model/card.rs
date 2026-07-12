use std::sync::Arc;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, prelude::FromRow};
use thiserror::Error;
use uuid::Uuid;

/// `Card` represents a card of Kanban Board
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct Card {
    pub id: String,                     // a unique identifier for the card
    pub column_id: String,              // relationship to the Kanban Board
    pub title: String,                  // title of the Kanban card
    pub description: Option<String>, // the main content of the card, this should be the persisted markup
    pub status: String,              // status of the card - 'Active', 'Blocked'
    pub blocked_reason: Option<String>, // reason for a card being 'Blocked'
    pub created_at: NaiveDateTime,
    pub started_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
}

/// `NewCard` represents a data request to create a new card on Kanban board
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewCard {
    pub column_id: String,              // relationship to the Kanban Board
    pub title: String,                  // title of the Kanban card
    pub description: Option<String>, // the main content of the card, this should be the persisted markup
    pub status: String,              // status of the card - 'Active', 'Blocked'
    pub blocked_reason: Option<String>, // reason for a card being 'Blocked'
}

impl NewCard {
    pub fn new(
        column_id: String,
        title: String,
        description: Option<String>,
        status: String,
        blocked_reason: Option<String>,
    ) -> Self {
        Self {
            column_id,
            title,
            description,
            status,
            blocked_reason,
        }
    }
}

/// `CardRepoError` represents the error enum related to error cases
/// that can happen at repository layer, we want specific error at repo level
/// due to easier option to handle error specific cases at higher service level
#[derive(Debug, Error)]
pub enum CardRepoError {
    #[error("card with id {id} not found")]
    NotFound { id: String },

    #[error("card with id {id} could not be deleted")]
    DeleteFailed { id: String },

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// specific result type related to `CardRepo`
pub type Result<T> = std::result::Result<T, CardRepoError>;

/// `CardRepo` represents the repository service layer responsible for managing the `Card` entities in the db
#[derive(Debug)]
pub struct CardRepo {
    pool: Arc<Pool<Sqlite>>, // reference to the DB connection pool
}

impl CardRepo {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn create_card(&self, card: NewCard) -> Result<Card> {
        let created_card = sqlx::query_as::<_, Card>(
            r#"
            INSERT INTO cards (id, column_id, title, description, status, blocked_reason)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING id, column_id, title, description, status, blocked_reason, created_at, started_at, completed_at
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&card.column_id)
        .bind(&card.title)
        .bind(&card.description)
        .bind(&card.status)
        .bind(&card.blocked_reason)
        .fetch_one(&*self.pool)
        .await?;
        Ok(created_card)
    }

    pub async fn get_card_by_id(&self, id: &str) -> Result<Card> {
        sqlx::query_as::<_, Card>(
            r#"
            SELECT id, column_id, title, description, status, blocked_reason, created_at, started_at, completed_at
            FROM cards
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or(CardRepoError::NotFound {
            id: String::from(id),
        })
    }

    pub async fn list_cards_for_column(&self, column_id: &str) -> Result<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT id, column_id, title, description, status, blocked_reason, created_at, started_at, completed_at
            FROM cards
            WHERE column_id = ?1
            ORDER BY title ASC
            "#,
        )
        .bind(column_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(cards)
    }

    pub async fn update_card(&self, card: Card) -> Result<Card> {
        sqlx::query_as::<_, Card>(
            r#"
            UPDATE cards
            SET column_id = ?1, title = ?2, description = ?3, status = ?4, blocked_reason = ?5, started_at = ?6, completed_at = ?7
            WHERE id = ?8
            RETURNING id, column_id, title, description, status, blocked_reason, created_at, started_at, completed_at
            "#,
        )
        .bind(&card.column_id)
        .bind(&card.title)
        .bind(&card.description)
        .bind(&card.status)
        .bind(&card.blocked_reason)
        .bind(card.started_at)
        .bind(card.completed_at)
        .bind(&card.id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or(CardRepoError::NotFound {
            id: String::from(&card.id),
        })
    }

    pub async fn delete_card_by_id(&self, id: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM cards
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await?;
        if result.rows_affected() == 0 {
            Err(CardRepoError::DeleteFailed {
                id: String::from(id),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{AppConfig, KanbanBoard};
    use crate::model::board::{BoardColumnRepo, NewBoardColumn};
    use crate::model::connection::Connection;
    use crate::model::test_utils::{acquire_test_lock, reset_db_pool};
    use anyhow::Result;
    use std::path::PathBuf;
    use std::{
        collections::HashMap,
        path::Path,
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;

    async fn init_repo(db_path: &Path) -> Result<CardRepo> {
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

        let connection = Connection::new(Arc::new(Mutex::new(config)));
        let pool = connection.get_db_connection_pool().await?;
        Ok(CardRepo::new(pool))
    }

    async fn insert_test_column(pool: Arc<Pool<Sqlite>>, name: &str) -> Result<String> {
        let column_repo = BoardColumnRepo::new(pool.clone());
        let created = column_repo
            .create_column(NewBoardColumn::new(String::from(name), 5, 0))
            .await?;
        Ok(created.id)
    }

    #[tokio::test]
    async fn create_card_inserts_and_returns_a_card() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_create.db");
        let repo = init_repo(&db_file).await?;

        let column_id = insert_test_column(repo.pool.clone(), "Column 1").await?;

        let created = repo
            .create_card(NewCard::new(
                column_id.clone(),
                String::from("Test card"),
                Some(String::from("Description")),
                String::from("Active"),
                None,
            ))
            .await?;

        assert_eq!(created.column_id, column_id);
        assert_eq!(created.title, "Test card");
        assert_eq!(created.status, "Active");
        Ok(())
    }

    #[tokio::test]
    async fn create_card_fail_when_column_missing() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_fk_create.db");
        let repo = init_repo(&db_file).await?;

        let error = repo
            .create_card(NewCard::new(
                String::from("missing-column"),
                String::from("Orphan card"),
                None,
                String::from("Active"),
                None,
            ))
            .await
            .expect_err("expected foreign key database error");

        if let CardRepoError::Database(sqlx_error) = error {
            let db_error = sqlx_error
                .as_database_error()
                .expect("expected a SQL database error");
            let message = db_error.message();
            assert!(message.contains("FOREIGN KEY"));
            assert!(message.contains("constraint"));
        } else {
            panic!("expected CardRepoError::Database");
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_card_by_id_returns_not_found_for_unknown_id() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_missing.db");
        let repo = init_repo(&db_file).await?;

        let error = repo
            .get_card_by_id("missing-id")
            .await
            .expect_err("expected not found");

        assert!(matches!(error, CardRepoError::NotFound { id } if id == "missing-id"));
        Ok(())
    }

    #[tokio::test]
    async fn list_cards_for_column_returns_cards_sorted_by_title() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_list.db");
        let repo = init_repo(&db_file).await?;

        let column_id = insert_test_column(repo.pool.clone(), "Column 2").await?;

        repo.create_card(NewCard::new(
            column_id.clone(),
            String::from("Zebra"),
            None,
            String::from("Active"),
            None,
        ))
        .await?;
        repo.create_card(NewCard::new(
            column_id.clone(),
            String::from("Alpha"),
            None,
            String::from("Blocked"),
            Some(String::from("Reason")),
        ))
        .await?;

        let cards = repo.list_cards_for_column(&column_id).await?;
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].title, "Alpha");
        assert_eq!(cards[1].title, "Zebra");
        Ok(())
    }

    #[tokio::test]
    async fn update_card_returns_not_found_when_missing() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_update_missing.db");
        let repo = init_repo(&db_file).await?;

        let missing_card = Card {
            id: String::from("missing-id"),
            column_id: String::from("column-3"),
            title: String::from("Missing"),
            description: None,
            status: String::from("Active"),
            blocked_reason: None,
            created_at: chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            started_at: None,
            completed_at: None,
        };

        let error = repo
            .update_card(missing_card)
            .await
            .expect_err("expected not found");

        assert!(matches!(error, CardRepoError::NotFound { id } if id == "missing-id"));
        Ok(())
    }

    #[tokio::test]
    async fn update_card_changes_existing_card() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_update.db");
        let repo = init_repo(&db_file).await?;

        let column_id = insert_test_column(repo.pool.clone(), "Column 4").await?;
        let created = repo
            .create_card(NewCard::new(
                column_id.clone(),
                String::from("Initial"),
                None,
                String::from("Active"),
                None,
            ))
            .await?;

        let updated = Card {
            title: String::from("Updated Title"),
            status: String::from("Blocked"),
            blocked_reason: Some(String::from("Blocked reason")),
            ..created
        };

        let result = repo.update_card(updated).await?;
        assert_eq!(result.title, "Updated Title");
        assert_eq!(result.status, "Blocked");
        assert_eq!(result.blocked_reason, Some(String::from("Blocked reason")));
        Ok(())
    }

    #[tokio::test]
    async fn update_card_fails_when_column_missing() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_fk_update.db");
        let repo = init_repo(&db_file).await?;

        let column_id = insert_test_column(repo.pool.clone(), "Column 5").await?;
        let created = repo
            .create_card(NewCard::new(
                column_id.clone(),
                String::from("Card"),
                None,
                String::from("Active"),
                None,
            ))
            .await?;

        let updated = Card {
            column_id: String::from("missing-column"),
            ..created
        };

        let error = repo
            .update_card(updated)
            .await
            .expect_err("expected fk error");

        if let CardRepoError::Database(sqlx_error) = error {
            let db_error = sqlx_error
                .as_database_error()
                .expect("expected a SQL database error");
            let message = db_error.message();
            assert!(message.contains("FOREIGN KEY"));
            assert!(message.contains("constraint"));
        } else {
            panic!("expected CardRepoError::Database");
        }

        Ok(())
    }

    #[tokio::test]
    async fn delete_card_by_id_returns_delete_failed_when_missing() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_delete_missing.db");
        let repo = init_repo(&db_file).await?;

        let error = repo
            .delete_card_by_id("missing-id")
            .await
            .expect_err("expected delete failed");

        assert!(matches!(error, CardRepoError::DeleteFailed { id } if id == "missing-id"));
        Ok(())
    }

    #[tokio::test]
    async fn delete_card_by_id_removes_existing_card() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("card_delete.db");
        let repo = init_repo(&db_file).await?;

        let column_id = insert_test_column(repo.pool.clone(), "Column 6").await?;
        let created = repo
            .create_card(NewCard::new(
                column_id.clone(),
                String::from("Delete me"),
                None,
                String::from("Active"),
                None,
            ))
            .await?;

        repo.delete_card_by_id(&created.id).await?;

        let error = repo
            .get_card_by_id(&created.id)
            .await
            .expect_err("expected not found");
        assert!(matches!(error, CardRepoError::NotFound { id } if id == created.id));
        Ok(())
    }
}
