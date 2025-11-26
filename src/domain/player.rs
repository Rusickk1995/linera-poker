use serde::{Deserialize, Serialize};

use crate::domain::card::Card;
use crate::domain::chips::Chips;
use crate::domain::PlayerId;

/// Базовый профиль игрока – то, что не зависит от конкретного стола/турнира.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerProfile {
    pub id: PlayerId,
    pub name: String,
}

/// Статус игрока именно в контексте стола/раздачи.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlayerStatus {
    /// Игрок активен в текущей раздаче.
    Active,
    /// Игрок сфолдил и больше не участвует в банке.
    Folded,
    /// Игрок в оллыне – не может больше делать ставки.
    AllIn,
    /// Игрок сидит за столом, но не участвует в раздаче (sit out).
    SittingOut,
    /// Игрок вылетел (нулевой стек в турнире или ушёл с кеш-стола).
    Busted,
}

/// Состояние игрока за конкретным столом.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerAtTable {
    pub player_id: PlayerId,
    /// Текущий стек за столом.
    pub stack: Chips,
    /// Ставка в текущем раунде (для удобства движка).
    pub current_bet: Chips,
    pub status: PlayerStatus,
    /// Карманные карты (0, 1 или 2 для холдема).
    pub hole_cards: Vec<Card>,
}

impl PlayerAtTable {
    pub fn new(player_id: PlayerId, stack: Chips) -> Self {
        Self {
            player_id,
            stack,
            current_bet: Chips::ZERO,
            status: PlayerStatus::Active,
            hole_cards: Vec::new(),
        }
    }

    pub fn is_in_hand(&self) -> bool {
        matches!(self.status, PlayerStatus::Active | PlayerStatus::AllIn)
    }
}
