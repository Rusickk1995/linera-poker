// src/time_ctrl/mod.rs
//! Подсистема тайминга (shot clock + таймбанк + перерывы).
//!
//! Это чистый оффчейн-модуль: не знает ни про столы, ни про турнир,
//! ни про блокчейн Linera. Его задача — считать секунды и говорить
//! дирижёру: «игрок X выгорел по времени, сделай auto-действие».

use serde::{Deserialize, Serialize};

use crate::domain::PlayerId;

mod clock;
mod extra_time;
mod time_bank;
mod time_rules;

pub use clock::{TimeoutState, TurnClock};
pub use extra_time::ExtraTimeGrant;
pub use time_bank::{PlayerTimeBank, TimeBank};
pub use time_rules::{TimeProfile, TimeRules};

/// Решение, которое тайм-контроллер предлагает движку/дирижёру.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AutoActionDecision {
    /// Ничего не делать — ход ещё в рамках времени.
    None,
    /// Игрок выгорел по времени, нужно применить auto-check / auto-fold.
    TimeoutCheckOrFold { player_id: PlayerId },
}

/// Основной контроллер времени для стола/турнира.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeController {
    pub rules: TimeRules,
    pub bank: TimeBank,
    pub clock: TurnClock,
}

impl TimeController {
    /// Создать контроллер для заданного профиля.
    pub fn new(profile: TimeProfile) -> Self {
        let rules = TimeRules::from_profile(profile);
        Self {
            rules,
            bank: TimeBank::new(),
            clock: TurnClock::new(),
        }
    }

    /// Создать контроллер с кастомными правилами.
    pub fn with_rules(rules: TimeRules) -> Self {
        Self {
            rules,
            bank: TimeBank::new(),
            clock: TurnClock::new(),
        }
    }

    /// Инициализировать таймбанк для набора игроков.
    ///
    /// Это удобно дергать из турнирного слоя, когда сформирован список участников.
    pub fn init_players<I>(&mut self, players: I)
    where
        I: IntoIterator<Item = PlayerId>,
    {
        self.bank.reset();
        self.bank.init_for_players(&self.rules, players);
    }

    /// Начать ход игрока, вернуть информацию о выданном extra-time.
    ///
    /// Дирижёр может показывать это на фронте (анимации таймбанка и т.п.).
    pub fn start_player_turn(&mut self, player_id: PlayerId) -> ExtraTimeGrant {
        self.clock
            .start_turn(player_id, &self.rules, &mut self.bank)
    }

    /// Сбросить текущий ход (завершили раздачу / перешли к следующему игроку вручную).
    pub fn clear_current_turn(&mut self) {
        self.clock.clear();
    }

    /// Сообщить, что прошло `delta_secs` секунд реального времени.
    ///
    /// Обычно дирижёр будет дергать это по тикам (например, раз в 1 секунду),
    /// после чего, если вернулся Timeout, он инициирует auto-check / auto-fold.
    pub fn on_time_passed(&mut self, delta_secs: i32) -> AutoActionDecision {
        use TimeoutState::*;

        match self
            .clock
            .elapse_for_current(delta_secs, &self.rules, &mut self.bank)
        {
            NoActivePlayer | Ongoing => AutoActionDecision::None,
            UsedExtraTime { .. } => {
                // Для продакшна тут можно логировать/метрики отправлять,
                // но движок действий не требует.
                AutoActionDecision::None
            }
            TimedOut { player_id } => AutoActionDecision::TimeoutCheckOrFold { player_id },
        }
    }

    /// Остаток таймбанка игрока (для отображения на фронте).
    pub fn remaining_bank_for(&self, player_id: PlayerId) -> i32 {
        self.bank.remaining_for(player_id)
    }
}
