use crate::domain::card::Rank;
use crate::domain::hand::HandRank;

/// Категория покерной руки по силе.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandCategory {
    HighCard = 0,
    OnePair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
}

impl HandRank {
    /// Собрать HandRank из категории и 5 рангов (от старшего к младшему).
    ///
    /// Схема кодирования (u32):
    ///   [категория:4 бита][r0:4][r1:4][r2:4][r3:4][r4:4]
    /// Rank: 2..14 (2..A) влазит в 4 бита.
    pub fn from_category_and_ranks(category: HandCategory, ranks: [Rank; 5]) -> Self {
        let cat_bits = (category as u32) & 0x0F;
        let r0 = rank_to_nibble(ranks[0]);
        let r1 = rank_to_nibble(ranks[1]);
        let r2 = rank_to_nibble(ranks[2]);
        let r3 = rank_to_nibble(ranks[3]);
        let r4 = rank_to_nibble(ranks[4]);

        let value = (cat_bits << 20)
            | ((r0 as u32) << 16)
            | ((r1 as u32) << 12)
            | ((r2 as u32) << 8)
            | ((r3 as u32) << 4)
            | (r4 as u32);

        HandRank(value)
    }

    /// Вытащить категорию из HandRank.
    pub fn category(&self) -> HandCategory {
        let cat_id = ((self.0 >> 20) & 0x0F) as u8;
        match cat_id {
            0 => HandCategory::HighCard,
            1 => HandCategory::OnePair,
            2 => HandCategory::TwoPair,
            3 => HandCategory::ThreeOfAKind,
            4 => HandCategory::Straight,
            5 => HandCategory::Flush,
            6 => HandCategory::FullHouse,
            7 => HandCategory::FourOfAKind,
            8 => HandCategory::StraightFlush,
            _ => HandCategory::HighCard,
        }
    }

    /// Достать 5 рангов (от старшего к младшему) из HandRank.
    pub fn ranks(&self) -> [Rank; 5] {
        let r0 = ((self.0 >> 16) & 0x0F) as u8;
        let r1 = ((self.0 >> 12) & 0x0F) as u8;
        let r2 = ((self.0 >> 8) & 0x0F) as u8;
        let r3 = ((self.0 >> 4) & 0x0F) as u8;
        let r4 = (self.0 & 0x0F) as u8;

        [
            nibble_to_rank(r0),
            nibble_to_rank(r1),
            nibble_to_rank(r2),
            nibble_to_rank(r3),
            nibble_to_rank(r4),
        ]
    }
}

fn rank_to_nibble(rank: Rank) -> u8 {
    // Rank::Two = 2, ..., Ace = 14 — всё помещается в 4 бита.
    rank as u8
}

fn nibble_to_rank(n: u8) -> Rank {
    match n {
        2 => Rank::Two,
        3 => Rank::Three,
        4 => Rank::Four,
        5 => Rank::Five,
        6 => Rank::Six,
        7 => Rank::Seven,
        8 => Rank::Eight,
        9 => Rank::Nine,
        10 => Rank::Ten,
        11 => Rank::Jack,
        12 => Rank::Queen,
        13 => Rank::King,
        14 => Rank::Ace,
        _ => Rank::Two, // fallback, но при корректной работе сюда не попадём
    }
}

/// Удобная функция – получить категорию из HandRank.
pub fn hand_category(rank: HandRank) -> HandCategory {
    rank.category()
}

/// Человеческое описание руки по категории.
/// (Детально раскрашивать по картам можно позже на уровне фронта).
pub fn describe_hand(rank: HandRank) -> String {
    let cat = rank.category();
    match cat {
        HandCategory::HighCard => "High card".to_string(),
        HandCategory::OnePair => "One pair".to_string(),
        HandCategory::TwoPair => "Two pair".to_string(),
        HandCategory::ThreeOfAKind => "Three of a kind".to_string(),
        HandCategory::Straight => "Straight".to_string(),
        HandCategory::Flush => "Flush".to_string(),
        HandCategory::FullHouse => "Full house".to_string(),
        HandCategory::FourOfAKind => "Four of a kind".to_string(),
        HandCategory::StraightFlush => "Straight flush".to_string(),
    }
}
