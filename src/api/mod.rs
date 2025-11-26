//! Внешний API покерного движка.
//!
//! Здесь описываются:
//! - команды (commands.rs) — всё, что меняет состояние (создать стол, посадить игрока, действие игрока);
//! - запросы (queries.rs) — только чтение;
//! - DTO (dto.rs) — удобные структуры для фронта;
//! - ошибки (errors.rs) — то, что видит клиент.

pub mod commands;
pub mod dto;
pub mod errors;
pub mod queries;

pub use commands::*;
pub use dto::*;
pub use errors::*;
pub use queries::*;
