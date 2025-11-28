// src/time_ctrl/time_rules.rs
//! Правила тайминга (shot clock + таймбанк).

use serde::{Deserialize, Serialize};

/// Профиль тайминга для турнира / стола.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeProfile {
    /// Стандартный MTT / кэш.
    Standard,
    /// Быстрый турбо-формат.
    Turbo,
    /// Глубокий, много времени на решения.
    Deep,
}

/// Конкретные параметры тайминга.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeRules {
    /// Базовое время на одно решение (секунды).
    pub base_action_secs: i32,
    /// Объём таймбанка на одного игрока (секунды, на весь турнир).
    pub bank_per_player_secs: i32,
    /// Шаг, с которым выдаётся extra-time на ход (секунды).
    ///
    /// Например: 20 секунд — значит, на ход можно запросить до 20 секунд
    /// из таймбанка. Следующие 20 выдаются только после исчерпания первых.
    pub bank_step_secs: i32,
}

impl TimeRules {
    pub const fn new(
        base_action_secs: i32,
        bank_per_player_secs: i32,
        bank_step_secs: i32,
    ) -> Self {
        Self {
            base_action_secs,
            bank_per_player_secs,
            bank_step_secs,
        }
    }

    /// Стандартный профиль: 20 сек + 60 сек таймбанка, шаг 20.
    pub const fn standard() -> Self {
        Self {
            base_action_secs: 20,
            bank_per_player_secs: 60,
            bank_step_secs: 20,
        }
    }

    /// Турбо-профиль: 10 сек + 30 сек таймбанка, шаг 10.
    pub const fn turbo() -> Self {
        Self {
            base_action_secs: 10,
            bank_per_player_secs: 30,
            bank_step_secs: 10,
        }
    }

    /// Глубокий: 30 сек + 120 сек таймбанка, шаг 30.
    pub const fn deep() -> Self {
        Self {
            base_action_secs: 30,
            bank_per_player_secs: 120,
            bank_step_secs: 30,
        }
    }

    /// Выбор правил по профилю.
    pub const fn from_profile(profile: TimeProfile) -> Self {
        match profile {
            TimeProfile::Standard => Self::standard(),
            TimeProfile::Turbo => Self::turbo(),
            TimeProfile::Deep => Self::deep(),
        }
    }
}
