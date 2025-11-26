use serde::{Deserialize, Serialize};

use crate::domain::{Chips, PlayerId, SeatIndex};

/// Тип действия игрока.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PlayerActionKind {
    Fold,
    Check,
    Call,
    /// Bet на новой улице (когда ещё нет текущей ставки).
    Bet(Chips),
    /// Raise существующей ставки.
    Raise(Chips),
    /// All-in – поставить весь стек.
    AllIn,
}

/// Конкретное действие игрока.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerAction {
    /// Какой игрок действует.
    pub player_id: PlayerId,
    /// В каком месте он сидит (0..max_seats-1).
    pub seat: SeatIndex,
    /// Само действие.
    pub kind: PlayerActionKind,
}
