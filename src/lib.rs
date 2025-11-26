//! Главный модуль приложения Poker на Linera.
//!
//! Здесь описываем ABI (Operation / Message / Query / Response) и
//! связываем contract/service с нашим PokerState.

pub mod infra;
pub mod api;
pub mod domain;
pub mod engine;
pub mod eval;
pub mod state;
pub mod tournament;
pub mod time_ctrl;

use linera_sdk::linera_base_types::{ContractAbi, ServiceAbi};
use serde::{Deserialize, Serialize};

use crate::api::{Command, Query, QueryResponse};
use crate::state::PokerState;

/// Операции (внешние команды), которые модуль принимает.
///
/// Для простоты: одна операция = одна команда из api::Command.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PokerOperation {
    Command(Command),
}

/// Сообщения между приложениями Linera.
/// Пока нам не нужны – оставим пустой enum.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PokerMessage {}

/// Запросы к сервису (read-only).
pub type PokerQuery = Query;

/// Ответы на запросы.
pub type PokerResponse = QueryResponse;

/// ABI для контракта и сервиса.
#[derive(Clone, Debug)]
pub struct PokerAbi;

impl ContractAbi for PokerAbi {
    type Operation = PokerOperation;
    type Response = ();
}

impl ServiceAbi for PokerAbi {
    type Query = PokerQuery;
    type QueryResponse = PokerResponse;
}

/// Экспортируем типы состояния, чтобы contract.rs и service.rs могли их использовать.
pub type Storage = PokerState;
