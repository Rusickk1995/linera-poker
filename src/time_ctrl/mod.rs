// src/time_ctrl/mod.rs
//! Вспомогательный модуль контроля времени (shot clock + time bank).
//!
//! Здесь собираем:
//! - правила (`TimeRules`);
//! - банк времени игроков (`TimeBank`);
//! - локальный таймер хода (`TurnClock`);
//! - фасад `TimeController`, который удобно использовать в рантайме турниров/столов.

pub mod clock;
pub mod extra_time;
pub mod time_bank;
pub mod time_rules;

pub use clock::{TimeoutState, TurnClock};
pub use extra_time::ExtraTimeGrant;
pub use time_bank::{PlayerTimeBank, TimeBank};
pub use time_rules::{TimeProfile, TimeRules};

use crate::domain::PlayerId;

/// Какое авто-действие нужно сделать при полном истечении времени.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutoActionDecision {
    /// Время не вышло — действий со стороны движка не требуется.
    None,
    /// Время полностью истекло — надо сделать AUTO CHECK / AUTO FOLD
    /// в зависимости от состояния стола (это уже решает покерный движок).
    TimeoutCheckOrFold,
}

/// Высокоуровневый контроллер времени для стола/турнира.
#[derive(Clone, Debug)]
pub struct TimeController {
    pub rules: TimeRules,
    pub bank: TimeBank,
    pub clock: TurnClock,
}

impl TimeController {
    /// Создать контроллер с заданными правилами (например, `TimeRules::standard()`).
    pub fn new(rules: TimeRules) -> Self {
        Self {
            rules,
            bank: TimeBank::new(),
            clock: TurnClock::new(),
        }
    }

    /// Инициализировать банк для набора игроков (обычно при старте турнира).
    pub fn init_players<I>(&mut self, players: I)
    where
        I: IntoIterator<Item = PlayerId>,
    {
        let initial = self.rules.bank_per_player_secs as i32;
        self.bank.init_for_players(players, initial);
    }

    /// Начать ход конкретного игрока.
    pub fn start_turn(&mut self, player_id: PlayerId) {
        self.clock.start_turn(player_id, &self.rules);
    }

    /// Сообщить контроллеру, что игрок успешно сделал действие вовремя.
    /// Это просто очищает таймер текущего хода.
    pub fn on_manual_action(&mut self, player_id: PlayerId) {
        if self.clock.current_player == Some(player_id) {
            self.clock.clear();
        }
    }

    /// "Протекание" времени для текущего актёра.
    ///
    /// `delta_secs` — сколько секунд прошло с последнего обновления (в онлайне — по wall-clock,
    /// в CLI/ботах можно симулировать фиксированное или случайное значение).
    ///
    /// Возвращаем, нужно ли триггерить авто-действие Check/Fold.
    pub fn on_time_passed(&mut self, delta_secs: i32) -> AutoActionDecision {
        match self.clock.elapse_for_current(delta_secs, &self.rules, &mut self.bank) {
            TimeoutState::Ongoing | TimeoutState::NoActivePlayer | TimeoutState::UsedExtraTime { .. } => {
                AutoActionDecision::None
            }
            TimeoutState::TimedOut => AutoActionDecision::TimeoutCheckOrFold,
        }
    }
}
