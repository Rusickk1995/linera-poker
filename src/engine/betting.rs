use serde::{Deserialize, Serialize};

use crate::domain::chips::Chips;
use crate::domain::hand::Street;
use crate::domain::SeatIndex;

/// Состояние раунда ставок (на конкретной улице).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BettingState {
    /// Текущая целевая ставка, до которой должны дотянуться игроки (BB, bet, raise).
    pub current_bet: Chips,
    /// Минимальный размер повышающей части рейза.
    pub min_raise: Chips,
    /// Seat последнего агрессора (bet/raise/all-in).
    pub last_aggressor: Option<SeatIndex>,
    /// Улица, к которой относится этот раунд.
    pub street: Street,
    /// Очередь ходящих (по кругу), кто ещё должен сделать действие на этой улице.
    pub to_act: Vec<SeatIndex>,
}

impl BettingState {
    pub fn new(street: Street, current_bet: Chips, min_raise: Chips, to_act: Vec<SeatIndex>) -> Self {
        Self {
            current_bet,
            min_raise,
            last_aggressor: None,
            street,
            to_act,
        }
    }

    /// Удалить seat из очереди to_act, если он там есть.
    pub fn mark_acted(&mut self, seat: SeatIndex) {
        self.to_act.retain(|s| *s != seat);
    }

    /// Обновить состояние после bet/raise:
    /// - current_bet
    /// - min_raise
    /// - last_aggressor
    /// - перезапустить очередь to_act (engine её сформирует).
    pub fn on_raise(&mut self, seat: SeatIndex, new_bet: Chips, raise_size: Chips, new_to_act: Vec<SeatIndex>) {
        self.current_bet = new_bet;
        self.min_raise = raise_size;
        self.last_aggressor = Some(seat);
        self.to_act = new_to_act;
    }

    /// Проверка, завершён ли раунд ставок:
    /// - очередь пуста.
    pub fn is_round_complete(&self) -> bool {
        self.to_act.is_empty()
    }
}
