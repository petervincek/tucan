//! Database connection pool management.
//!
//! This module provides a singleton pattern for managing SQLite database connections
//! using a thread-safe connection pool with lazy initialization and automatic migration support.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use sqlx::migrate::MigrateError;
use sqlx::pool::PoolConnectionMetadata;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Sqlite, SqliteConnection, SqlitePool};
use thiserror::Error;
use tracing::debug;

use crate::core::config::AppConfig;

/// Lazily initialized SQLite connection pool wrapped in Arc for thread-safe sharing.
static DB_POOL: Mutex<Option<Arc<SqlitePool>>> = Mutex::new(None);

/// Lock used to synchronize pool initialization to ensure only one thread initializes it.
static INIT_LOCK: Mutex<()> = Mutex::new(());

/// `PoolError` represents the error enum related to error cases
/// that can happen at this layer that establishes db connection
/// we want to have very specific errors at this level so higher level
/// services can potentially react to these specific errors (the context is needed)
/// type clarity of the error is the benefit
#[derive(Debug, Error)]
pub enum PoolError {
    #[error("Pool can not be created, error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Can not load config, error: {0}")]
    Migration(#[from] MigrateError),
    #[error("Connection not initialized [{reason}]")]
    ConnectionNotInitialized { reason: String },
}

pub struct Connection {
    pub config: Arc<Mutex<AppConfig>>,
}

/// specific result type related to `Connection` abstraction
pub type Result<T> = std::result::Result<T, PoolError>;

impl Connection {
    pub fn new(config: Arc<Mutex<AppConfig>>) -> Self {
        Self { config }
    }

    pub fn reset_db_pool_for_tests() {
        let _ = DB_POOL.lock().unwrap().take();
    }

    pub async fn get_db_connection_pool(&self) -> Result<Arc<Pool<Sqlite>>> {
        let _guard = INIT_LOCK.lock().unwrap();

        if let Some(pool) = DB_POOL.lock().unwrap().as_ref() {
            return Ok(pool.clone());
        }

        let config = self.config.lock().unwrap();
        let current_board_id = &config.current_board;
        if let Some(kanban_board) = config.kanban_boards.get(current_board_id) {
            // IMPORTANT: SQLite does NOT enforce foreign key constraints (including ON DELETE CASCADE)
            // unless PRAGMA foreign_keys = ON is set for every new connection. This is required even if
            // your schema defines foreign keys. The after_connect hook below ensures that foreign key
            // enforcement is enabled for every pooled connection, so that referential integrity and
            // cascade behaviors work as expected throughout the application.
            let db_url = kanban_board.db_url.clone();
            // let db_url = db_url.into_os_string();
            let options = SqliteConnectOptions::new()
                .filename(normalize_sqlite_file_path(&db_url))
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal);
            // set the logging
            // options = options.log_statements(log::LevelFilter::Off);
            let db_pool = SqlitePoolOptions::new()
                .after_connect(enable_sqlite_foreign_keys())
                .connect_with(options)
                .await?;
            let pool = Arc::new(db_pool);
            *DB_POOL.lock().unwrap() = Some(pool.clone());
            run_migrations(&pool).await?;
            Ok(pool)
        } else {
            Err(PoolError::ConnectionNotInitialized {
                reason: format!("connection details are missing for board id: {current_board_id}"),
            })
        }
    }
}

pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    debug!("Run migrations scripts successful.");
    Ok(())
}

fn normalize_sqlite_file_path(db_url: &std::path::Path) -> std::path::PathBuf {
    let db_url_str = db_url.to_string_lossy();
    if let Some(stripped) = db_url_str.strip_prefix("sqlite://") {
        PathBuf::from(stripped)
    } else {
        db_url.to_path_buf()
    }
}

type SqliteAfterConnect = dyn for<'c> Fn(
        &'c mut SqliteConnection,
        PoolConnectionMetadata,
    ) -> Pin<Box<dyn Future<Output = sqlx::Result<()>> + Send + 'c>>
    + Send
    + Sync;

// Define a reusable callback for after_connect
pub fn enable_sqlite_foreign_keys() -> Box<SqliteAfterConnect> {
    Box::new(|conn, _meta| {
        Box::pin(async move {
            sqlx::query("PRAGMA foreign_keys = ON;")
                .execute(conn)
                .await?;
            Ok(())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{AppConfig, KanbanBoard};
    use crate::model::test_utils::{acquire_test_lock, reset_db_pool};
    use anyhow::Result;
    use std::path::PathBuf;
    use std::{collections::HashMap, fs, path::Path};
    use tempfile::tempdir;

    fn build_test_config(db_path: &Path) -> AppConfig {
        let mut boards = HashMap::new();
        boards.insert(
            String::from("test-board"),
            KanbanBoard::new(
                String::from("Test Board"),
                String::from("A temporary board for tests"),
                PathBuf::from(format!("sqlite://{}", db_path.display())),
            ),
        );

        AppConfig {
            current_board: String::from("test-board"),
            logging_config: Default::default(),
            kanban_boards: boards,
        }
    }

    #[tokio::test]
    async fn get_db_connection_pool_returns_pool_for_valid_board_config() -> Result<()> {
        let _lock = acquire_test_lock();
        reset_db_pool();

        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("tucan_test.db");
        let config = build_test_config(&db_file);
        let connection = Connection::new(Arc::new(Mutex::new(config)));

        let pool = connection.get_db_connection_pool().await?;

        let columns_table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='columns'",
        )
        .fetch_one(&*pool)
        .await?;
        let cards_table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='cards'",
        )
        .fetch_one(&*pool)
        .await?;

        assert_eq!(
            columns_table_count, 1,
            "expecting to have one SQLite table: columns"
        );
        assert_eq!(
            cards_table_count, 1,
            "expecting to have one SQLite table: cards"
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_db_connection_pool_reuses_existing_pool() -> Result<()> {
        let _lock = acquire_test_lock();
        reset_db_pool();

        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("reuse_pool.db");
        let config = build_test_config(&db_file);
        let connection = Connection::new(Arc::new(Mutex::new(config)));

        let first_pool = connection.get_db_connection_pool().await?;
        let second_pool = connection.get_db_connection_pool().await?;

        assert!(Arc::ptr_eq(&first_pool, &second_pool));
        Ok(())
    }

    #[tokio::test]
    async fn get_db_connection_pool_handles_sqlite_url_prefix_in_path() -> Result<()> {
        let _lock = acquire_test_lock();
        reset_db_pool();

        let temp_dir = tempdir()?;
        let db_path = temp_dir
            .path()
            .join("sqlite://marker")
            .join("tucan_test.db");
        fs::create_dir_all(db_path.parent().unwrap())?;

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

        let columns_table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND (name='columns' OR name='cards')",
        )
        .fetch_one(&*pool)
        .await?;
        assert_eq!(columns_table_count, 2);

        Ok(())
    }

    #[tokio::test]
    async fn get_db_connection_pool_errors_when_current_board_is_missing() -> Result<()> {
        reset_db_pool();

        let config = AppConfig {
            current_board: String::from("missing-board"),
            logging_config: Default::default(),
            kanban_boards: HashMap::new(),
        };
        let connection = Connection::new(Arc::new(Mutex::new(config)));

        let error = connection
            .get_db_connection_pool()
            .await
            .expect_err("expected an error");

        assert!(matches!(error, PoolError::ConnectionNotInitialized { .. }));
        Ok(())
    }

    #[tokio::test]
    async fn run_migrations_creates_expected_tables() -> Result<()> {
        reset_db_pool();

        let temp_dir = tempdir()?;
        let db_file = temp_dir.path().join("migrations_test.db");
        let options = SqliteConnectOptions::new()
            .filename(db_file.to_string_lossy().to_string())
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);

        let sqlite_pool = SqlitePoolOptions::new()
            .after_connect(enable_sqlite_foreign_keys())
            .connect_with(options)
            .await?;
        let pool = Arc::new(sqlite_pool);

        run_migrations(&*pool).await?;

        let cards_table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND (name='cards' OR name='columns')",
        )
        .fetch_one(&*pool)
        .await?;
        assert_eq!(cards_table_count, 2);

        Ok(())
    }
}
