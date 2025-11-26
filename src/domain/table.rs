use serde::{Deserialize, Serialize};

use crate::domain::blinds::{AnteType};
use crate::domain::card::Card;
use crate::domain::chips::Chips;
use crate::domain::hand::Street;
use crate::domain::player::PlayerAtTable;
use crate::domain::{HandId, TableId};

/// Индекс места за столом (0..max_seats-1).
pub type SeatIndex = u8;

/// Тип стола.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TableType {
    Cash,
    Tournament,
}

/// Конфиг стола: сколько мест, какие лимиты, анте и т.д.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableConfig {
    /// Максимальное количество мест за столом (обычно 2–9).
    pub max_seats: u8,
    pub table_type: TableType,
    /// Размеры блайндов/анте для кеш-стола или стартовые для турнира.
    pub stakes: TableStakes,
    /// Разрешён ли страддл и другие расширения – флаги на будущее.
    pub allow_straddle: bool,
    /// Разрешён ли run-it-twice и т.п. – это уже доп.функционал.
    pub allow_run_it_twice: bool,
}

/// Стейки стола (SB/BB/ante).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableStakes {
    pub small_blind: Chips,
    pub big_blind: Chips,
    pub ante_type: AnteType,
    pub ante: Chips,
}

impl TableStakes {
    pub fn new(sb: Chips, bb: Chips, ante_type: AnteType, ante: Chips) -> Self {
        Self {
            small_blind: sb,
            big_blind: bb,
            ante_type,
            ante,
        }
    }
}

/// Основное состояние стола.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Table {
    pub id: TableId,
    pub name: String,
    pub config: TableConfig,

    /// Места за столом: индекс вектора = SeatIndex.
    /// None – место пустое.
    pub seats: Vec<Option<PlayerAtTable>>,

    /// Общие карты борда (0–5 карт).
    pub board: Vec<Card>,

    /// Индекс дилерской кнопки (место дилера) или None, если раздача ещё не начиналась.
    pub dealer_button: Option<SeatIndex>,

    /// ID текущей раздачи (если она идёт).
    pub current_hand_id: Option<HandId>,

    /// Текущая улица раздачи.
    pub street: Street,

    /// Идёт ли сейчас раздача (true), либо стол ждёт начала новой.
    pub hand_in_progress: bool,

    /// Общий размер банка (без детализации по сайд-потам – это работа engine).
    pub total_pot: Chips,
}

impl Table {
    /// Создать пустой стол с заданной конфигурацией.
    pub fn new(id: TableId, name: String, config: TableConfig) -> Self {
        let seats = vec![None; config.max_seats as usize];
        Self {
            id,
            name,
            config,
            seats,
            board: Vec::new(),
            dealer_button: None,
            current_hand_id: None,
            street: Street::Preflop,
            hand_in_progress: false,
            total_pot: Chips::ZERO,
        }
    }

    pub fn max_seats(&self) -> u8 {
        self.config.max_seats
    }

    pub fn seated_count(&self) -> usize {
        self.seats.iter().filter(|s| s.is_some()).count()
    }

    pub fn is_seat_empty(&self, index: SeatIndex) -> bool {
        self.seats
            .get(index as usize)
            .map(|s| s.is_none())
            .unwrap_or(true)
    }
}
