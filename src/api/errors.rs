use serde::{Deserialize, Serialize};

use crate::domain::{PlayerId, TableId};
use crate::engine::EngineError;

/// Ошибки внешнего API (то, что отдаём фронту / клиенту).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ApiError {
    /// Неправильные входные данные (например, битый JSON).
    BadRequest(String),

    /// Стол не найден.
    TableNotFound(TableId),

    /// Игрок не найден за столом.
    PlayerNotAtTable(PlayerId),

    /// Команда не может быть выполнена в текущем состоянии.
    InvalidCommand(String),

    /// Ошибка движка (ставки, действия).
    EngineError(String),

    /// Внутренняя ошибка сервера.
    Internal(String),
}

impl From<EngineError> for ApiError {
    fn from(err: EngineError) -> Self {
        ApiError::EngineError(err.to_string())
    }
}
