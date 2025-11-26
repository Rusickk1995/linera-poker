// src/time_ctrl/clock.rs
//! Локальный таймер хода (shot clock) для текущего игрока.

use serde::{Deserialize, Serialize};

use crate::domain::PlayerId;

use super::{TimeBank, TimeRules};

/// Состояние таймера текущего хода.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnClock {
    /// Какой игрок сейчас должен сделать ход (None, если сейчас нет активного хода).
    pub current_player: Option<PlayerId>,
    /// Сколько секунд базового времени ещё осталось на этот ход.
    pub remaining_action_secs: i32,
    /// Сколько секунд дополнительного времени (из банка) ещё осталось на этот ход.
    pub remaining_extra_secs: i32,
}

/// Результат "протекания" времени.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimeoutState {
    /// Время ещё не вышло, игрок может думать дальше.
    Ongoing,
    /// Был подключён таймбанк (выдали extra-time), но игрок ещё не вылетел по времени.
    UsedExtraTime { granted_secs: i32 },
    /// Базовое время и таймбанк для этого игрока полностью исчерпаны —
    /// надо авто-Check/авто-Fold.
    TimedOut,
    /// Сейчас нет активного игрока, на кого вешать таймер.
    NoActivePlayer,
}

impl TurnClock {
    pub fn new() -> Self {
        Self {
            current_player: None,
            remaining_action_secs: 0,
            remaining_extra_secs: 0,
        }
    }

    /// Начать ход нового игрока согласно правилам.
    pub fn start_turn(&mut self, player_id: PlayerId, rules: &TimeRules) {
        self.current_player = Some(player_id);
        self.remaining_action_secs = rules.base_action_secs as i32;
        self.remaining_extra_secs = 0;
    }

    /// Очистить состояние таймера (например, после того, как игрок сделал действие).
    pub fn clear(&mut self) {
        self.current_player = None;
        self.remaining_action_secs = 0;
        self.remaining_extra_secs = 0;
    }

    /// Симулируем протекание `delta_secs` времени для текущего игрока.
    ///
    /// Логика:
    /// - сначала тратим base-время;
    /// - когда оно ушло, пытаемся подключить extra-time из таймбанка (кусок bank_step_secs);
    /// - если таймбанк тоже исчерпан и ещё остался `delta` — считаем, что игрок вылетел по времени.
    pub fn elapse_for_current(
        &mut self,
        delta_secs: i32,
        rules: &TimeRules,
        bank: &mut TimeBank,
    ) -> TimeoutState {
        if delta_secs <= 0 {
            return TimeoutState::Ongoing;
        }

        let player_id = match self.current_player {
            Some(pid) => pid,
            None => return TimeoutState::NoActivePlayer,
        };

        let mut remaining = delta_secs;

        // 1. Тратим оставшееся базовое время
        if self.remaining_action_secs > 0 {
            if remaining < self.remaining_action_secs {
                self.remaining_action_secs -= remaining;
                return TimeoutState::Ongoing;
            } else {
                remaining -= self.remaining_action_secs;
                self.remaining_action_secs = 0;
            }
        }

        // 2. Если базовое время закончилось, но ещё не подключали extra,
        //    пробуем выдать "шаг" из таймбанка.
        if self.remaining_extra_secs <= 0 {
            let granted = bank.grant_from_bank(player_id, rules.bank_step_secs as i32);
            if granted > 0 {
                self.remaining_extra_secs = granted;
                // Если overflow от base-времени был 0 — просто сообщаем, что подключили extra.
                if remaining == 0 {
                    return TimeoutState::UsedExtraTime { granted_secs: granted };
                }
            } else {
                // Таймбанк нулевой и base-время уже 0 — любая дополнительная секунда = таймаут.
                return TimeoutState::TimedOut;
            }
        }

        // 3. Тратим extra-time
        if remaining < self.remaining_extra_secs {
            self.remaining_extra_secs -= remaining;
            TimeoutState::Ongoing
        } else {
            // Доп. время тоже проели.
            self.remaining_extra_secs = 0;
            TimeoutState::TimedOut
        }
    }
}
