use core::ops::{Add, AddAssign, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// Количество фишек. Обёртка над u64, чтобы не путать с обычными числами.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Chips(pub u64);

impl Chips {
    pub const ZERO: Chips = Chips(0);

    pub fn new(amount: u64) -> Self {
        Chips(amount)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Безопасное вычитание, не даёт уйти в минус.
    pub fn saturating_sub(self, other: Chips) -> Chips {
        Chips(self.0.saturating_sub(other.0))
    }
}

impl Add for Chips {
    type Output = Chips;

    fn add(self, rhs: Chips) -> Self::Output {
        Chips(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for Chips {
    fn add_assign(&mut self, rhs: Chips) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl Sub for Chips {
    type Output = Chips;

    fn sub(self, rhs: Chips) -> Self::Output {
        Chips(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for Chips {
    fn sub_assign(&mut self, rhs: Chips) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}
