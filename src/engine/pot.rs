use serde::{Deserialize, Serialize};

use crate::domain::chips::Chips;

/// Главный (total) банк. Детализацию по сайд-потам делаем отдельно.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pot {
    pub total: Chips,
}

impl Pot {
    pub fn new() -> Self {
        Self {
            total: Chips::ZERO,
        }
    }

    pub fn add(&mut self, amount: Chips) {
        self.total += amount;
    }

    pub fn reset(&mut self) {
        self.total = Chips::ZERO;
    }
}
