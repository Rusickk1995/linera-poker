use serde::{Deserialize, Serialize};

use crate::domain::chips::Chips;

/// Тип анте в турнире/кеше.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnteType {
    /// Без анте.
    None,
    /// Классическое анте с каждого игрока.
    Classic,
    /// Big Blind Ante – анте платит только биг-блайнд.
    BigBlind,
}

/// Один уровень блайндов.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlindLevel {
    /// Номер уровня (1,2,3,...).
    pub level: u32,
    pub small_blind: Chips,
    pub big_blind: Chips,
    /// Сумма анте (если есть).
    pub ante: Chips,
    pub ante_type: AnteType,
    /// Длительность уровня в минутах (для турнирной структуры).
    pub duration_minutes: u32,
}

impl BlindLevel {
    pub fn new(
        level: u32,
        sb: Chips,
        bb: Chips,
        ante: Chips,
        ante_type: AnteType,
        duration_minutes: u32,
    ) -> Self {
        Self {
            level,
            small_blind: sb,
            big_blind: bb,
            ante,
            ante_type,
            duration_minutes,
        }
    }
}

/// Полная структура блайндов (турнир).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlindStructure {
    pub levels: Vec<BlindLevel>,
}

impl BlindStructure {
    pub fn new(levels: Vec<BlindLevel>) -> Self {
        Self { levels }
    }

    pub fn first_level(&self) -> Option<&BlindLevel> {
        self.levels.first()
    }

    /// Получить уровень по его номеру (если нет – последний).
    pub fn level_by_number(&self, level: u32) -> Option<&BlindLevel> {
        self.levels.iter().find(|l| l.level == level)
    }

    /// Простейшая логика: считаем уровень по прошедшим минутам.
    /// Engine может использовать это как основу и накручивать сверху более сложные правила.
    pub fn level_for_elapsed_minutes(&self, minutes: u32) -> Option<&BlindLevel> {
        if self.levels.is_empty() {
            return None;
        }

        let mut acc = 0;
        for level in &self.levels {
            acc += level.duration_minutes;
            if minutes < acc {
                return Some(level);
            }
        }

        // Если время превысило все уровни – возвращаем последний (блайнды не растут дальше).
        self.levels.last()
    }
}

impl Default for BlindStructure {
    fn default() -> Self {
        use crate::domain::chips::Chips;
        use AnteType::*;

        let levels = vec![
            BlindLevel {
                level: 1,
                small_blind: Chips::new(25),
                big_blind: Chips::new(50),
                ante: Chips::ZERO,
                ante_type: None,
                duration_minutes: 10,
            },
            BlindLevel {
                level: 2,
                small_blind: Chips::new(50),
                big_blind: Chips::new(100),
                ante: Chips::ZERO,
                ante_type: None,
                duration_minutes: 10,
            },
        ];

        BlindStructure { levels }
    }
}

