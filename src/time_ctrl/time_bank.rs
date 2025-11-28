// src/time_ctrl/time_bank.rs
//! Таймбанк игроков: сколько секунд дополнительного времени у кого осталось.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::PlayerId;
use super::TimeRules;

/// Таймбанк одного игрока (секунды).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerTimeBank {
    pub remaining_secs: i32,
}

impl PlayerTimeBank {
    pub fn new(initial_secs: i32) -> Self {
        Self {
            remaining_secs: initial_secs.max(0),
        }
    }

    /// Выдать `requested` секунд из таймбанка.
    /// Возвращает фактически выданное (может быть меньше, если банк пустеет).
    pub fn grant(&mut self, requested: i32) -> i32 {
        if requested <= 0 || self.remaining_secs <= 0 {
            return 0;
        }
        let grant = requested.min(self.remaining_secs);
        self.remaining_secs -= grant;
        grant
    }

    /// Добавить секунды в банк (для бонусов, промо и т.п.).
    pub fn add(&mut self, secs: i32) {
        if secs <= 0 {
            return;
        }
        self.remaining_secs = self.remaining_secs.saturating_add(secs);
    }
}

/// Глобальный таймбанк турнира / стола.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimeBank {
    players: HashMap<PlayerId, PlayerTimeBank>,
}

impl TimeBank {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }

    /// Полностью очистить таймбанк (например, новый турнир).
    pub fn reset(&mut self) {
        self.players.clear();
    }

    /// Инициализировать таймбанк для набора игроков.
    pub fn init_for_players<I>(&mut self, rules: &TimeRules, players: I)
    where
        I: IntoIterator<Item = PlayerId>,
    {
        let initial = rules.bank_per_player_secs.max(0);
        for pid in players {
            self.players
                .entry(pid)
                .or_insert_with(|| PlayerTimeBank::new(initial));
        }
    }

    /// Добавить игроку времени в банк.
    pub fn add_time(&mut self, player_id: PlayerId, secs: i32) {
        if secs <= 0 {
            return;
        }
        self.players
            .entry(player_id)
            .or_insert_with(|| PlayerTimeBank::new(0))
            .add(secs);
    }

    /// Выдать `requested` секунд extra-time для текущего хода игрока.
    pub fn grant_for_turn(&mut self, player_id: PlayerId, requested: i32) -> i32 {
        if requested <= 0 {
            return 0;
        }
        if let Some(bank) = self.players.get_mut(&player_id) {
            bank.grant(requested)
        } else {
            0
        }
    }

    /// Остаток таймбанка у игрока (для отображения на фронте).
    pub fn remaining_for(&self, player_id: PlayerId) -> i32 {
        self.players
            .get(&player_id)
            .map(|b| b.remaining_secs)
            .unwrap_or(0)
    }
}
