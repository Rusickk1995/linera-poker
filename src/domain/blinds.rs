// src/domain/blinds.rs

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
/// Пример: level = 3, SB = 100, BB = 200, ante = 25, ante_type = BigBlind, duration_minutes = 10.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlindLevel {
    /// Порядковый номер уровня (1, 2, 3, ...).
    pub level: u32,
    /// Малый блайнд.
    pub small_blind: Chips,
    /// Большой блайнд.
    pub big_blind: Chips,
    /// Размер анте в фишках (0, если нет).
    pub ante: Chips,
    /// Тип анте: None / Classic / BigBlind.
    pub ante_type: AnteType,
    /// Длительность уровня в минутах.
    pub duration_minutes: u32,
}

impl BlindLevel {
    pub fn new(
        level: u32,
        small_blind: Chips,
        big_blind: Chips,
        ante: Chips,
        ante_type: AnteType,
        duration_minutes: u32,
    ) -> Self {
        Self {
            level,
            small_blind,
            big_blind,
            ante,
            ante_type,
            duration_minutes,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.small_blind.0 == 0 {
            return Err(format!("BlindLevel {}: small_blind = 0", self.level));
        }
        if self.big_blind.0 == 0 {
            return Err(format!("BlindLevel {}: big_blind = 0", self.level));
        }
        if self.big_blind.0 <= self.small_blind.0 {
            return Err(format!(
                "BlindLevel {}: big_blind ({}) <= small_blind ({})",
                self.level, self.big_blind.0, self.small_blind.0
            ));
        }
        if self.duration_minutes == 0 {
            return Err(format!(
                "BlindLevel {}: duration_minutes = 0",
                self.level
            ));
        }
        Ok(())
    }
}

/// Структура уровней блайндов для турнира.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlindStructure {
    pub levels: Vec<BlindLevel>,
}

impl BlindStructure {
    pub fn new(levels: Vec<BlindLevel>) -> Self {
        Self { levels }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.levels.is_empty() {
            return Err("BlindStructure: empty levels".into());
        }

        let mut expected_level = 1u32;
        for lvl in &self.levels {
            lvl.validate()?;
            if lvl.level != expected_level {
                return Err(format!(
                    "BlindStructure: expected level {}, got {}",
                    expected_level, lvl.level
                ));
            }
            expected_level += 1;
        }

        Ok(())
    }

    pub fn first_level(&self) -> &BlindLevel {
        &self.levels[0]
    }

    pub fn level_by_number(&self, number: u32) -> Option<&BlindLevel> {
        self.levels.iter().find(|lvl| lvl.level == number)
    }

    pub fn total_duration_minutes(&self) -> u32 {
        self.levels
            .iter()
            .map(|lvl| lvl.duration_minutes)
            .sum()
    }

    /// elasped_minutes считается от момента старта турнира (не учитывая перерывы).
    pub fn level_for_elapsed_minutes(&self, elapsed_minutes: u32) -> &BlindLevel {
        let mut acc = 0u32;
        let mut current = &self.levels[0];

        for lvl in &self.levels {
            acc += lvl.duration_minutes;
            if elapsed_minutes < acc {
                return lvl;
            }
            current = lvl;
        }

        current
    }

    pub fn simple_demo_structure() -> Self {
        let levels = vec![
            BlindLevel::new(
                1,
                Chips::new(25),
                Chips::new(50),
                Chips::ZERO,
                AnteType::None,
                10,
            ),
            BlindLevel::new(
                2,
                Chips::new(50),
                Chips::new(100),
                Chips::ZERO,
                AnteType::None,
                10,
            ),
            BlindLevel::new(
                3,
                Chips::new(75),
                Chips::new(150),
                Chips::new(25),
                AnteType::BigBlind,
                10,
            ),
        ];

        BlindStructure { levels }
    }
}
