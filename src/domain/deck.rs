use serde::{Deserialize, Serialize};

use crate::domain::card::{Card, Rank, Suit};

/// Колода карт. В домене — просто упорядоченный список карт.
/// Перемешивание делает engine (через RNG из infra), НЕ здесь.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    /// Стандартная 52-карточная колода в порядке:
    /// Clubs 2..A, Diamonds 2..A, Hearts 2..A, Spades 2..A.
    pub fn standard_52() -> Self {
        let mut cards = Vec::with_capacity(52);
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            for rank in [
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::Ace,
            ] {
                cards.push(Card::new(rank, suit));
            }
        }
        Deck { cards }
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Взять одну карту сверху колоды.
    pub fn draw_one(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    /// Взять n карт сверху.
    pub fn draw_n(&mut self, n: usize) -> Vec<Card> {
        let mut taken = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(card) = self.cards.pop() {
                taken.push(card);
            } else {
                break;
            }
        }
        taken
    }

    /// Убрать из колоды уже использованные карты (для безопасности).
    pub fn remove_cards(&mut self, to_remove: &[Card]) {
        self.cards
            .retain(|c| !to_remove.iter().any(|r| r == c));
    }
}
