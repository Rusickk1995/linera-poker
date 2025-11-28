// src/time_ctrl/clock.rs
//! Turn clock (shot clock) для одного стола.

use serde::{Deserialize, Serialize};

use crate::domain::PlayerId;

use super::{ExtraTimeGrant, TimeBank, TimeRules};

/// Состояние таймера после "протекания" времени.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeoutState {
    /// Сейчас нет активного игрока (ход не у кого).
    NoActivePlayer,
    /// Всё ещё в пределах допустимого времени.
    Ongoing,
    /// Было сожжено extra-time, но лимит ещё не исчерпан.
    UsedExtraTime {
        player_id: PlayerId,
        used_secs: i32,
    },
    /// Время полностью вышло — требуется авто-действие (check/fold).
    TimedOut {
        player_id: PlayerId,
    },
}

/// Внутренний turn-clock для одного стола.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnClock {
    /// Текущий игрок, который должен принять решение.
    pub current_player: Option<PlayerId>,
    /// Оставшееся базовое время на ход (секунды).
    pub remaining_action_secs: i32,
    /// Оставшееся extra-time на этот ход (секунды).
    pub remaining_extra_secs: i32,
}

impl TurnClock {
    pub fn new() -> Self {
        Self {
            current_player: None,
            remaining_action_secs: 0,
            remaining_extra_secs: 0,
        }
    }

    /// Сбросить текущий ход (например, после завершения раздачи).
    pub fn clear(&mut self) {
        self.current_player = None;
        self.remaining_action_secs = 0;
        self.remaining_extra_secs = 0;
    }

    /// Начинается ход игрока.
    ///
    /// - базовое время берётся из `rules.base_action_secs`;
    /// - сразу выдаём один "пакет" extra-time из таймбанка (bank_step_secs);
    /// - возвращаем `ExtraTimeGrant`, чтобы фронт мог показать анимацию / индикатор.
    pub fn start_turn(
        &mut self,
        player_id: PlayerId,
        rules: &TimeRules,
        bank: &mut TimeBank,
    ) -> ExtraTimeGrant {
        self.current_player = Some(player_id);
        self.remaining_action_secs = rules.base_action_secs.max(0);

        let step = rules.bank_step_secs.max(0);
        let granted = if step > 0 {
            bank.grant_for_turn(player_id, step)
        } else {
            0
        };

        self.remaining_extra_secs = granted;

        if granted > 0 {
            ExtraTimeGrant::new(granted)
        } else {
            ExtraTimeGrant::none()
        }
    }

    /// Сообщить часам, что прошло `delta_secs` секунд.
    ///
    /// Возвращает:
    /// - `Ongoing`, если время ещё есть;
    /// - `UsedExtraTime`, если сгорела часть extra-time;
    /// - `TimedOut`, если вообще всё время вышло.
    pub fn elapse_for_current(
        &mut self,
        delta_secs: i32,
        rules: &TimeRules,
        bank: &mut TimeBank,
    ) -> TimeoutState {
        let player_id = match self.current_player {
            Some(pid) => pid,
            None => return TimeoutState::NoActivePlayer,
        };

        if delta_secs <= 0 {
            return TimeoutState::Ongoing;
        }

        let mut remaining = delta_secs;
        let mut used_extra: i32 = 0;

        // 1) Жжём базовое время на ход.
        if self.remaining_action_secs > 0 {
            let burn = remaining.min(self.remaining_action_secs);
            self.remaining_action_secs -= burn;
            remaining -= burn;
        }

        if remaining <= 0 {
            return TimeoutState::Ongoing;
        }

        // 2) Жжём уже выданное extra-time.
        if self.remaining_extra_secs > 0 {
            let burn = remaining.min(self.remaining_extra_secs);
            self.remaining_extra_secs -= burn;
            used_extra += burn;
            remaining -= burn;
        }

        if remaining <= 0 {
            return if used_extra > 0 {
                TimeoutState::UsedExtraTime {
                    player_id,
                    used_secs: used_extra,
                }
            } else {
                TimeoutState::Ongoing
            };
        }

        // 3) Если всё ещё не хватило — пробуем подтянуть ещё пакеты из таймбанка.
        while remaining > 0 {
            let step = rules.bank_step_secs;
            if step <= 0 {
                break;
            }

            let granted = bank.grant_for_turn(player_id, step);
            if granted <= 0 {
                break;
            }

            let burn = remaining.min(granted);
            used_extra += burn;
            remaining -= burn;

            if burn < granted {
                // Часть extra-time ещё осталась на этот ход.
                self.remaining_extra_secs = granted - burn;
                break;
            }
        }

        if remaining > 0 {
            // Время кончилось полностью.
            self.remaining_action_secs = 0;
            self.remaining_extra_secs = 0;
            TimeoutState::TimedOut { player_id }
        } else if used_extra > 0 {
            TimeoutState::UsedExtraTime {
                player_id,
                used_secs: used_extra,
            }
        } else {
            TimeoutState::Ongoing
        }
    }
}
