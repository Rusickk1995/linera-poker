use crate::domain::card::{Card, Rank, Suit};
use crate::domain::hand::HandRank;

use super::hand_rank::{HandCategory};
use super::lookup_tables::{detect_straight, rank_to_bit, RankMask};

/// Расширяем HandRank методами из eval (чтобы удобнее было внутри).
trait HandRankExt {
    fn from_category_and_ranks(category: HandCategory, ranks: [Rank; 5]) -> Self;
}

impl HandRankExt for HandRank {
    fn from_category_and_ranks(category: HandCategory, ranks: [Rank; 5]) -> Self {
        HandRank::from_category_and_ranks(category, ranks)
    }
}

/// Главная функция: вычислить лучшую 5-карточную руку из hole + board.
///
/// Ожидается:
///   - `hole.len() == 2`
///   - `board.len()` от 3 до 5 (обычно 5)
///
/// Но в целом функция корректно работает для любых 5–7 карт.
pub fn evaluate_best_hand(hole: &[Card], board: &[Card]) -> HandRank {
    let mut all_cards = Vec::with_capacity(hole.len() + board.len());
    all_cards.extend_from_slice(hole);
    all_cards.extend_from_slice(board);

    assert!(
        (5..=7).contains(&all_cards.len()),
        "evaluate_best_hand ожидает от 5 до 7 карт"
    );

    best_of_all_5card_combinations(&all_cards)
}

/// Перебираем все комбинации 5 карт из N (N=5–7) и выбираем лучшую.
fn best_of_all_5card_combinations(cards: &[Card]) -> HandRank {
    let n = cards.len();
    assert!(n >= 5 && n <= 7);

    let mut best: Option<HandRank> = None;

    for a in 0..(n - 4) {
        for b in (a + 1)..(n - 3) {
            for c in (b + 1)..(n - 2) {
                for d in (c + 1)..(n - 1) {
                    for e in (d + 1)..n {
                        let five = [
                            cards[a],
                            cards[b],
                            cards[c],
                            cards[d],
                            cards[e],
                        ];
                        let r = evaluate_5card_hand(&five);
                        if best.map_or(true, |best_r| r > best_r) {
                            best = Some(r);
                        }
                    }
                }
            }
        }
    }

    best.expect("должна быть хотя бы одна 5-карточная комбинация")
}

/// Оценка строго 5-карточной комбинации.
fn evaluate_5card_hand(cards: &[Card; 5]) -> HandRank {
    // Подсчёт мастей.
    let mut suit_counts = [0u8; 4]; // 0:clubs,1:diamonds,2:hearts,3:spades

    // Подсчёт рангов.
    let mut rank_counts = [0u8; 15]; // индексы 0..14, но используем 2..14
    let mut rank_mask: RankMask = 0;

    for card in cards.iter() {
        let suit_idx = match card.suit {
            Suit::Clubs => 0,
            Suit::Diamonds => 1,
            Suit::Hearts => 2,
            Suit::Spades => 3,
        };
        suit_counts[suit_idx] += 1;

        let r_val = card.rank as usize;
        rank_counts[r_val] += 1;
        rank_mask |= rank_to_bit(card.rank);
    }

    let is_flush = suit_counts.iter().any(|&c| c == 5);
    let straight_high_rank = detect_straight(rank_mask);

    // Список (rank, count) для анализа пар/сет/каре.
    #[derive(Clone, Copy)]
    struct RankCount {
        rank: Rank,
        count: u8,
    }

    let mut rc_list: Vec<RankCount> = Vec::with_capacity(5);
    for r_val in (2usize..=14usize).rev() {
        let c = rank_counts[r_val];
        if c > 0 {
            let rank = num_to_rank(r_val as u8);
            rc_list.push(RankCount { rank, count: c });
        }
    }

    // Сортируем сначала по количеству (desc), затем по рангу (desc).
    rc_list.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| b.rank.cmp(&a.rank))
    });

    // pattern counts: например [4,1], [3,2], [3,1,1], [2,2,1], [2,1,1,1], [1,1,1,1,1]
    let pattern: Vec<u8> = rc_list.iter().map(|rc| rc.count).collect();

    // Проверка на straight flush.
    if is_flush {
        if let Some(high) = straight_high_rank {
            // Straight flush.
            let ranks = straight_rank_array(high);
            return HandRank::from_category_and_ranks(HandCategory::StraightFlush, ranks);
        }
    }

    // Four of a kind.
    if pattern == [4, 1] {
        let four = rc_list[0].rank;
        let kicker = rc_list[1].rank;
        let ranks = [four, kicker, Rank::Two, Rank::Two, Rank::Two];
        // последние 3 ранга можно забить "мусором" (они не сравниваются)
        return HandRank::from_category_and_ranks(HandCategory::FourOfAKind, ranks);
    }

    // Full house: 3+2
    if pattern == [3, 2] {
        let trips = rc_list[0].rank;
        let pair = rc_list[1].rank;
        let ranks = [trips, pair, Rank::Two, Rank::Two, Rank::Two];
        return HandRank::from_category_and_ranks(HandCategory::FullHouse, ranks);
    }

    // Flush.
    if is_flush {
        // Берём 5 карт flush'а, отсортированных по убыванию ранга.
        let mut flush_cards: Vec<Card> = cards.to_vec();
        flush_cards.sort_by(|a, b| b.rank.cmp(&a.rank));
        let ranks = [
            flush_cards[0].rank,
            flush_cards[1].rank,
            flush_cards[2].rank,
            flush_cards[3].rank,
            flush_cards[4].rank,
        ];
        return HandRank::from_category_and_ranks(HandCategory::Flush, ranks);
    }

    // Straight.
    if let Some(high) = straight_high_rank {
        let ranks = straight_rank_array(high);
        return HandRank::from_category_and_ranks(HandCategory::Straight, ranks);
    }

    // Three of a kind (сет/трипс): 3+1+1
    if pattern == [3, 1, 1] {
        let trips = rc_list[0].rank;
        let kicker1 = rc_list[1].rank;
        let kicker2 = rc_list[2].rank;
        let ranks = [trips, kicker1, kicker2, Rank::Two, Rank::Two];
        return HandRank::from_category_and_ranks(HandCategory::ThreeOfAKind, ranks);
    }

    // Two pair: 2+2+1
    if pattern == [2, 2, 1] {
        let pair1 = rc_list[0].rank;
        let pair2 = rc_list[1].rank;
        let kicker = rc_list[2].rank;
        let ranks = [pair1, pair2, kicker, Rank::Two, Rank::Two];
        return HandRank::from_category_and_ranks(HandCategory::TwoPair, ranks);
    }

    // One pair: 2+1+1+1
    if pattern == [2, 1, 1, 1] {
        let pair = rc_list[0].rank;
        let kicker1 = rc_list[1].rank;
        let kicker2 = rc_list[2].rank;
        let kicker3 = rc_list[3].rank;
        let ranks = [pair, kicker1, kicker2, kicker3, Rank::Two];
        return HandRank::from_category_and_ranks(HandCategory::OnePair, ranks);
    }

    // High card: 1+1+1+1+1
    // Просто берём топ-5 рангов по убыванию.
    let mut ranks_only: Vec<Rank> = rc_list.iter().map(|rc| rc.rank).collect();
    ranks_only.sort_by(|a, b| b.cmp(a));
    while ranks_only.len() < 5 {
        ranks_only.push(Rank::Two);
    }
    let ranks = [
        ranks_only[0],
        ranks_only[1],
        ranks_only[2],
        ranks_only[3],
        ranks_only[4],
    ];
    HandRank::from_category_and_ranks(HandCategory::HighCard, ranks)
}

/// Построить массив рангов [r0..r4] для стрита с заданной старшей картой.
fn straight_rank_array(high: Rank) -> [Rank; 5] {
    match high {
        Rank::Five => [
            Rank::Five,
            Rank::Four,
            Rank::Three,
            Rank::Two,
            Rank::Ace, // wheel: A2345
        ],
        Rank::Six => [Rank::Six, Rank::Five, Rank::Four, Rank::Three, Rank::Two],
        Rank::Seven => [Rank::Seven, Rank::Six, Rank::Five, Rank::Four, Rank::Three],
        Rank::Eight => [Rank::Eight, Rank::Seven, Rank::Six, Rank::Five, Rank::Four],
        Rank::Nine => [Rank::Nine, Rank::Eight, Rank::Seven, Rank::Six, Rank::Five],
        Rank::Ten => [Rank::Ten, Rank::Nine, Rank::Eight, Rank::Seven, Rank::Six],
        Rank::Jack => [Rank::Jack, Rank::Ten, Rank::Nine, Rank::Eight, Rank::Seven],
        Rank::Queen => [Rank::Queen, Rank::Jack, Rank::Ten, Rank::Nine, Rank::Eight],
        Rank::King => [Rank::King, Rank::Queen, Rank::Jack, Rank::Ten, Rank::Nine],
        Rank::Ace => [Rank::Ace, Rank::King, Rank::Queen, Rank::Jack, Rank::Ten],
        _ => [high, Rank::Four, Rank::Three, Rank::Two, Rank::Two],
    }
}

fn num_to_rank(v: u8) -> Rank {
    match v {
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
        _ => Rank::Two,
    }
}
