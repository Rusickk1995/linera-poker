// src/time_ctrl/time_bank.rs
//! Time bank игроков (общий запас доп. времени на турнир/сессию).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::PlayerId;

/// Состояние таймбанка конкретного игрока.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerTimeBank {
    /// Сколько секунд банка ещё осталось (может стать 0, но не должно быть < 0).
    pub remaining_secs: i32,
}

impl PlayerTimeBank {
    pub fn new(initial_secs: i32) -> Self {
        Self {
            remaining_secs: initial_secs,
        }
    }

    /// Сколько осталось.
    pub fn remaining(&self) -> i32 {
        self.remaining_secs
    }

    /// Снять `requested` секунд из банка (если столько нет — отдаём сколько есть).
    /// Возвращаем реально выданное количество секунд (0, если банка больше нет).
    pub fn take(&mut self, requested: i32) -> i32 {
        if self.remaining_secs <= 0 || requested <= 0 {
            return 0;
        }

        let grant = self.remaining_secs.min(requested);
        self.remaining_secs -= grant;
        grant
    }
}

/// Общий time bank для всех игроков стола/турнира.
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

    /// Инициализировать/переинициализировать банк для заданного игрока.
    pub fn set_for_player(&mut self, player_id: PlayerId, initial_secs: i32) {
        self.players
            .insert(player_id, PlayerTimeBank::new(initial_secs));
    }

    /// Инициализировать банк для множества игроков.
    pub fn init_for_players<I>(&mut self, players: I, initial_secs: i32)
    where
        I: IntoIterator<Item = PlayerId>,
    {
        for pid in players {
            self.set_for_player(pid, initial_secs);
        }
    }

    /// Сколько секунд ещё есть у игрока.
    pub fn remaining_for(&self, player_id: PlayerId) -> i32 {
        self.players
            .get(&player_id)
            .map(|p| p.remaining())
            .unwrap_or(0)
    }

    /// Выдать игроку до `requested` секунд из таймбанка.
    /// Если банка нет — вернётся 0.
    pub fn grant_from_bank(&mut self, player_id: PlayerId, requested: i32) -> i32 {
        if requested <= 0 {
            return 0;
        }

        if let Some(p) = self.players.get_mut(&player_id) {
            p.take(requested)
        } else {
            0
        }
    }
}
