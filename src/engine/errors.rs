use crate::domain::{PlayerId, SeatIndex, TableId};

use thiserror::Error;

/// Ошибки движка покера.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Стол {0} не найден")]
    TableNotFound(TableId),

    #[error("Место {0} не существует за столом")]
    InvalidSeat(SeatIndex),

    #[error("В этом месте нет игрока")]
    EmptySeat,

    #[error("Игрок {0} не найден за столом")]
    PlayerNotAtTable(PlayerId),

    #[error("Недостаточно активных игроков для раздачи")]
    NotEnoughPlayers,

    #[error("Раздача уже идёт")]
    HandAlreadyInProgress,

    #[error("Раздача не активна")]
    NoActiveHand,

    #[error("Сейчас не ход игрока с id={0}")]
    NotPlayersTurn(PlayerId),

    #[error("Недопустимое действие в текущем состоянии раздачи")]
    IllegalAction,

    #[error("Недостаточно фишек для этой ставки")]
    NotEnoughChips,

    #[error("Размер рейза слишком мал")]
    RaiseTooSmall,

    #[error("Невозможно выполнить check – нужно хотя бы уравнять ставку")]
    CannotCheck,

    #[error("Невозможно выполнить call – нет ставки для уравнивания")]
    CannotCall,

    #[error("Внутренняя ошибка: {0}")]
    Internal(&'static str),
}
