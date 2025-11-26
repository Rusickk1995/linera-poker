use serde::{Deserialize, Serialize};

use crate::domain::card::Card;
use crate::domain::chips::Chips;
use crate::domain::{HandId, PlayerId, SeatIndex, TableId};
use crate::engine::actions::PlayerActionKind;
use crate::domain::hand::Street;

/// Тип события в раздаче.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HandEventKind {
    /// Новая раздача началась.
    HandStarted {
        table_id: TableId,
        hand_id: HandId,
    },

    /// Кнопка/блайнды.
    BlindsPosted {
        dealer: SeatIndex,
        small_blind: Option<(SeatIndex, Chips)>,
        big_blind: Option<(SeatIndex, Chips)>,
        ante: Vec<(SeatIndex, Chips)>,
    },

    /// Игрок получил карманные карты.
    HoleCardsDealt {
        seat: SeatIndex,
        cards: Vec<Card>,
    },

    /// Открыты общие карты на борде.
    BoardDealt {
        street: Street,
        cards: Vec<Card>,
    },

    /// Действие игрока.
    PlayerActed {
        player_id: PlayerId,
        seat: SeatIndex,
        action: PlayerActionKind,
        new_stack: Chips,
        pot_after: Chips,
    },

    /// Переход на новую улицу.
    StreetChanged {
        street: Street,
    },

    /// Шоудаун – открытие карт.
    ShowdownReveal {
        seat: SeatIndex,
        player_id: PlayerId,
        hole_cards: Vec<Card>,
        rank_value: u32,
    },

    /// Выплата банка(ов).
    PotAwarded {
        seat: SeatIndex,
        player_id: PlayerId,
        amount: Chips,
    },

    /// Раздача завершена.
    HandFinished {
        hand_id: HandId,
        table_id: TableId,
    },
}

/// Событие в раздаче с порядковым номером.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HandEvent {
    pub index: u32,
    pub kind: HandEventKind,
}

/// Полная история раздачи.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HandHistory {
    pub events: Vec<HandEvent>,
}

impl HandHistory {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, kind: HandEventKind) {
        let idx = self.events.len() as u32;
        self.events.push(HandEvent { index: idx, kind });
    }
}
