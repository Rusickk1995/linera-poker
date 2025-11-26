// src/time_ctrl/time_rules.rs
//! Конфигурация тайминга (shot-clock) для покера.
//!
//! Здесь описываем только "правила", без состояния и без привязки к конкретному столу.

use serde::{Deserialize, Serialize};

/// Профиль тайминга (на будущее можно добавить Turbo/Hyper и т.д.).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeProfile {
    /// Стандартный MTT / кэш: 20 сек на ход + time bank по 10 сек.
    Standard,
}

/// Правила тайминга для одного стола/турнира.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeRules {
    /// Сколько секунд даётся на базовое решение (каждый ход).
    pub base_action_secs: u32,
    /// Сколько секунд time bank доступно каждому игроку на турнир/сессию.
    pub bank_per_player_secs: u32,
    /// Какой "кусок" банка выдаётся за раз, когда base-время кончилось.
    pub bank_step_secs: u32,
}

impl TimeRules {
    /// Строгий конструктор.
    pub const fn new(base_action_secs: u32, bank_per_player_secs: u32, bank_step_secs: u32) -> Self {
        Self {
            base_action_secs,
            bank_per_player_secs,
            bank_step_secs,
        }
    }

    /// Стандартный профиль: 20 сек на ход, 60 сек банка, выдаём по 10 сек.
    pub const fn standard() -> Self {
        Self {
            base_action_secs: 20,
            bank_per_player_secs: 60,
            bank_step_secs: 10,
        }
    }

    /// Получить правила по профилю (например, Standard/Turbo).
    pub const fn from_profile(profile: TimeProfile) -> Self {
        match profile {
            TimeProfile::Standard => Self::standard(),
        }
    }
}
