use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, prelude::FromRow};
use thiserror::Error;
use uuid::Uuid;

/// `BoardColumn` represents a column of Kanban Board
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct BoardColumn {
    pub id: String,     // a unique identifier for the column
    pub name: String,   // column name
    pub wip_limit: u16, // Work In Progress limit for a given column
    pub position: u16,  // position of a column relative to the board (left -> right)
    pub created_at: NaiveDateTime,
}

/// `NewBoardColumn` represents a data request to create a new board column
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewBoardColumn {
    pub name: String,   // column name
    pub wip_limit: u16, // Work In Progress limit for a given column
    pub position: u16,  // position of a column relative to the board (left -> right)
}

impl NewBoardColumn {
    pub fn new(name: String, wip_limit: u16, position: u16) -> Self {
        Self {
            name,
            wip_limit,
            position,
        }
    }
}

/// `BoardColumnRepoError` represents the error enum related to error cases
/// that can happen at repository layer, we want specific error at repo level
/// due to easier option to handle error specific cases at higher service level
#[derive(Debug, Error)]
pub enum BoardColumnRepoError {
    #[error("board column with id {id} not found")]
    NotFound { id: String },

    #[error("board column with id {id} could not be deleted")]
    DeleteFailed { id: String },

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// specific result type related to `BoardColumnRepo`
pub type Result<T> = std::result::Result<T, BoardColumnRepoError>;

/// `BoardColumnRepo` represents the repository service layer responsible for managing the `BoardColumn` entities in the db
#[derive(Debug)]
pub struct BoardColumnRepo {
    pool: Pool<Sqlite>, // reference to the DB connection pool
}

impl BoardColumnRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create_column(&self, column: NewBoardColumn) -> Result<BoardColumn> {
        let created_board_column = sqlx::query_as::<_, BoardColumn>(
            r#"
            INSERT INTO columns (id, name, wip_limit, position)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING id, name, wip_limit, position, created_at
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&column.name)
        .bind(column.wip_limit)
        .bind(column.position)
        .fetch_one(&self.pool)
        .await?;
        Ok(created_board_column)
    }

    pub async fn get_column_by_id(&self, id: &str) -> Result<BoardColumn> {
        sqlx::query_as::<_, BoardColumn>(
            r#"
            SELECT id, name, wip_limit, position, created_at
            FROM columns
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(BoardColumnRepoError::NotFound {
            id: String::from(id),
        })
    }

    pub async fn list_columns(&self) -> Result<Vec<BoardColumn>> {
        let columns = sqlx::query_as::<_, BoardColumn>(
            r#"
            SELECT id, name, wip_limit, position, created_at
            FROM columns
            ORDER BY position ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(columns)
    }

    pub async fn update_column(&self, column: BoardColumn) -> Result<BoardColumn> {
        sqlx::query_as::<_, BoardColumn>(
            r#"
            UPDATE columns
            SET name = ?1, wip_limit = ?2, position =?3
            WHERE id = ?4
            RETURNING id, name, wip_limit, position, created_at
            "#,
        )
        .bind(&column.name)
        .bind(column.wip_limit)
        .bind(column.position)
        .bind(&column.id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(BoardColumnRepoError::NotFound {
            id: String::from(&column.id),
        })
    }

    pub async fn delete_column_by_id(&self, id: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM columns
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            Err(BoardColumnRepoError::DeleteFailed {
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
    use crate::model::connection::Connection;
    use crate::model::test_utils::{acquire_test_lock, reset_db_pool};
    use anyhow::Result;
    use std::{
        collections::HashMap,
        path::Path,
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;

    async fn init_repo(db_path: &Path) -> Result<BoardColumnRepo> {
        let _lock = acquire_test_lock(); // our unit tests run in parallel
        reset_db_pool();

        let mut boards = HashMap::new();
        boards.insert(
            String::from("test-board"),
            KanbanBoard::new(
                String::from("Test Board"),
                String::from("A temporary board for tests"),
                format!("sqlite://{}", db_path.display()),
            ),
        );

        let config = AppConfig {
            current_board: String::from("test-board"),
            logging_config: Default::default(),
            kanban_boards: boards,
        };

        let connection = Connection::new(Arc::new(Mutex::new(config)));
        let pool_arc = connection.get_db_connection_pool().await?;
        let pool = (*pool_arc).clone();

        Ok(BoardColumnRepo::new(pool))
    }

    #[tokio::test]
    async fn create_column_inserts_and_returns_a_column() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_create.db");
        let repo = init_repo(&db_file).await?;

        let created = repo
            .create_column(NewBoardColumn::new(String::from("Todo"), 3, 0))
            .await?;

        assert_eq!(created.name, "Todo");
        assert_eq!(created.wip_limit, 3);
        assert_eq!(created.position, 0);
        assert!(!created.id.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn get_column_by_id_returns_not_found_for_unknown_id() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_missing.db");
        let repo = init_repo(&db_file).await?;

        let error = repo
            .get_column_by_id("non-existing-id")
            .await
            .expect_err("expected not found error");

        assert!(matches!(error, BoardColumnRepoError::NotFound { id } if id == "non-existing-id"));
        Ok(())
    }

    #[tokio::test]
    async fn list_columns_returns_columns_in_ascending_position() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_list.db");
        let repo = init_repo(&db_file).await?;

        repo.create_column(NewBoardColumn::new(String::from("Done"), 2, 2))
            .await?;
        repo.create_column(NewBoardColumn::new(String::from("Todo"), 5, 0))
            .await?;
        repo.create_column(NewBoardColumn::new(String::from("Doing"), 4, 1))
            .await?;

        let columns = repo.list_columns().await?;

        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0].position, 0);
        assert_eq!(columns[0].name, "Todo");
        assert_eq!(columns[1].position, 1);
        assert_eq!(columns[2].position, 2);
        Ok(())
    }

    #[tokio::test]
    async fn update_column_returns_not_found_when_missing() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_update_missing.db");
        let repo = init_repo(&db_file).await?;

        let missing_column = BoardColumn {
            id: String::from("missing-id"),
            name: String::from("Blocked"),
            wip_limit: 1,
            position: 3,
            created_at: chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        };

        let error = repo
            .update_column(missing_column)
            .await
            .expect_err("expected not found");

        assert!(matches!(error, BoardColumnRepoError::NotFound { id } if id == "missing-id"));
        Ok(())
    }

    #[tokio::test]
    async fn update_column_changes_existing_column() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_update.db");
        let repo = init_repo(&db_file).await?;

        let created = repo
            .create_column(NewBoardColumn::new(String::from("Todo"), 3, 0))
            .await?;

        let updated = BoardColumn {
            id: created.id.clone(),
            name: String::from("Ready"),
            wip_limit: created.wip_limit,
            position: created.position,
            created_at: created.created_at,
        };

        let result = repo.update_column(updated).await?;
        assert_eq!(result.name, "Ready");
        Ok(())
    }

    #[tokio::test]
    async fn delete_column_by_id_returns_delete_failed_when_missing() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_delete_missing.db");
        let repo = init_repo(&db_file).await?;

        let error = repo
            .delete_column_by_id("missing-id")
            .await
            .expect_err("expected delete failed");

        assert!(matches!(error, BoardColumnRepoError::DeleteFailed { id } if id == "missing-id"));
        Ok(())
    }

    #[tokio::test]
    async fn delete_column_by_id_removes_existing_column() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_delete.db");
        let repo = init_repo(&db_file).await?;

        let created = repo
            .create_column(NewBoardColumn::new(String::from("Todo"), 3, 0))
            .await?;

        repo.delete_column_by_id(&created.id).await?;

        let error = repo
            .get_column_by_id(&created.id)
            .await
            .expect_err("expected not found");
        assert!(matches!(error, BoardColumnRepoError::NotFound { id } if id == created.id));
        Ok(())
    }

    #[tokio::test]
    async fn create_column_returns_database_error_for_duplicate_name() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_duplicate_name.db");
        let repo = init_repo(&db_file).await?;

        repo.create_column(NewBoardColumn::new(String::from("Todo"), 3, 0))
            .await?;

        let error = repo
            .create_column(NewBoardColumn::new(String::from("Todo"), 5, 1))
            .await
            .expect_err("expected duplicate name database error");

        assert!(matches!(error, BoardColumnRepoError::Database(_)));
        Ok(())
    }

    #[tokio::test]
    async fn list_columns_returns_empty_list_when_no_columns() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("board_list_empty.db");
        let repo = init_repo(&db_file).await?;

        let columns = repo.list_columns().await?;

        assert!(columns.is_empty());
        Ok(())
    }
}
