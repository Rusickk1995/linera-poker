use serde::{Deserialize, Serialize};

use crate::domain::card::Card;
use crate::domain::chips::Chips;
use crate::domain::{HandId, PlayerId, TableId};

/// Улица раздачи.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}

/// Ранг руки. Пока просто u32 – потом eval будет заполнять этот тип.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct HandRank(pub u32);

/// Результат конкретного игрока в раздаче.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerHandResult {
    pub player_id: PlayerId,
    /// Итоговый ранг руки (если дошёл до шоудауна).
    pub rank: Option<HandRank>,
    /// Сколько фишек выиграл/проиграл относительно начала раздачи.
    /// Положительное значение = выигрыш, отрицательное = потеря.
    pub net_chips: Chips,
    /// Является ли игрок победителем (включая сплит).
    pub is_winner: bool,
}

/// Краткое описание завершённой раздачи. Удобно для истории/реплеера.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandSummary {
    pub hand_id: HandId,
    pub table_id: TableId,
    pub street_reached: Street,
    pub board: Vec<Card>,
    pub total_pot: Chips,
    pub results: Vec<PlayerHandResult>,
}
