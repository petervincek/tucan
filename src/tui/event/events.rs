use anyhow::Error;

use crate::model::{
    board::{BoardColumn, BoardColumnRepoError},
    card::{Card, CardRepoError},
};

/// `AppEvent` enum represents all the application events/messages
/// used for async communication between the main terminal/rendering loop
/// and any async task that is offloaded to another thread -> I/O task, network calls etc.
#[derive(Debug)]
pub enum AppEvent {
    Config(ConfigEvent),
    Board(BoardEvent),
    Card(CardEvent),
}

/// `ConfigEvent` enum represent config specific events supported by our application
#[derive(Debug)]
pub enum ConfigEvent {
    ConfigPersisted,
    Error(Error),
}

/// `BoardEvent` enum represent events related to activities around Kanban board entities
#[derive(Debug)]
pub enum BoardEvent {
    BoardColumnCreated(BoardColumn),
    BoardColumnFetched(BoardColumn),
    BoardColumnsFetched(Vec<BoardColumn>),
    BoardColumnUpdated(BoardColumn),
    BoardColumnDeleted(String),
    Error(BoardColumnRepoError),
}

/// `CardEvent` enum represent event related to activities around Kanban card entities
#[derive(Debug)]
pub enum CardEvent {
    CardCreated(Card),
    CardFetched(Card),
    CardsFetched(Vec<Card>),
    CardUpdated(Card),
    CardDeleted(String),
    Error(CardRepoError),
}
