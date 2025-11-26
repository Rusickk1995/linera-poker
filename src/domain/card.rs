use core::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Масть карты.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Suit {
    Clubs,    // ♣
    Diamonds, // ♦
    Hearts,   // ♥
    Spades,   // ♠
}

/// Ранг карты.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Rank {
    Two = 2,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

/// Обычная покерная карта (52-карточная колода).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub const fn new(rank: Rank, suit: Suit) -> Self {
        Self { rank, suit }
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ch = match self {
            Suit::Clubs => 'c',
            Suit::Diamonds => 'd',
            Suit::Hearts => 'h',
            Suit::Spades => 's',
        };
        write!(f, "{ch}")
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ch = match self {
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
            r => char::from_digit(*r as u32, 10).unwrap(),
        };
        write!(f, "{ch}")
    }
}

impl fmt::Display for Card {
    /// Формат вида `Ah`, `Td`, `7c`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

/// Парсинг строки вида "Ah", "Td", "7c".
impl FromStr for Card {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 2 {
            return Err("Card string must have length 2".into());
        }
        let mut chars = s.chars();
        let r_ch = chars.next().unwrap();
        let s_ch = chars.next().unwrap();

        let rank = match r_ch {
            '2' => Rank::Two,
            '3' => Rank::Three,
            '4' => Rank::Four,
            '5' => Rank::Five,
            '6' => Rank::Six,
            '7' => Rank::Seven,
            '8' => Rank::Eight,
            '9' => Rank::Nine,
            'T' | 't' => Rank::Ten,
            'J' | 'j' => Rank::Jack,
            'Q' | 'q' => Rank::Queen,
            'K' | 'k' => Rank::King,
            'A' | 'a' => Rank::Ace,
            _ => return Err(format!("Invalid rank: {r_ch}")),
        };

        let suit = match s_ch {
            'c' | 'C' => Suit::Clubs,
            'd' | 'D' => Suit::Diamonds,
            'h' | 'H' => Suit::Hearts,
            's' | 'S' => Suit::Spades,
            _ => return Err(format!("Invalid suit: {s_ch}")),
        };

        Ok(Card { rank, suit })
    }
}
