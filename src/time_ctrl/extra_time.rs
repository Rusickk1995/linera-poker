// src/time_ctrl/extra_time.rs
//! Объект "выданное дополнительное время" для одного хода.

use serde::{Deserialize, Serialize};

/// Информация о выданном extra-time (на конкретный ход).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtraTimeGrant {
    /// Сколько секунд было добавлено к текущему ходу.
    pub granted_secs: i32,
}

impl ExtraTimeGrant {
    pub fn none() -> Self {
        Self { granted_secs: 0 }
    }

    pub fn new(granted_secs: i32) -> Self {
        Self { granted_secs }
    }

    pub fn is_active(&self) -> bool {
        self.granted_secs > 0
    }
}
