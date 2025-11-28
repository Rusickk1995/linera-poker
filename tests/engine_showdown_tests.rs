//! Showdown / hand evaluation tests для poker-engine.
//!
//! Здесь мы проверяем именно часть "кто сильнее на шоудауне":
//! - evaluate_best_hand для разных комбинаций;
//! - сравнение HandRank (>, ==, <);
//! - случаи split pot (одинаковая лучшая рука у двух игроков);
//! - кейс с кикером (у кого старше).

use poker_engine::domain::card::{Card, Rank, Suit};
use poker_engine::eval::evaluate_best_hand;

// Подтягиваем конструктор вариантов Rank::* и Suit::* в область видимости.
use Rank::*;
use Suit::*;

/// Удобный конструктор карты.
fn c(rank: Rank, suit: Suit) -> Card {
    Card { rank, suit }
}

//
// ============= ТЕСТ 1: straight flush > four of a kind ============
//
#[test]
fn straight_flush_beats_four_of_a_kind() {
    // Борд: 9♣, T♣, J♣, Q♣, 2♦
    let board = vec![
        c(Nine, Clubs),
        c(Ten, Clubs),
        c(Jack, Clubs),
        c(Queen, Clubs),
        c(Two, Diamonds),
    ];

    // Игрок 1: 8♣, K♣ → straight flush
    let p1_hole = vec![c(Eight, Clubs), c(King, Clubs)];

    // Игрок 2: K♦, K♥ → quads K
    let p2_hole = vec![c(King, Diamonds), c(King, Hearts)];

    let r1 = evaluate_best_hand(&p1_hole, &board);
    let r2 = evaluate_best_hand(&p2_hole, &board);

    assert!(r1 > r2, "Straight flush должен быть сильнее four of a kind");
}

//
// ============= ТЕСТ 2: four of a kind > full house ============
//
#[test]
fn four_of_a_kind_beats_full_house() {
    // Борд: K♣, K♦, 3♣, 3♦, 7♠
    let board = vec![
        c(King, Clubs),
        c(King, Diamonds),
        c(Three, Clubs),
        c(Three, Diamonds),
        c(Seven, Spades),
    ];

    // Игрок 1: K♥, K♠ → quads K
    let p1_hole = vec![c(King, Hearts), c(King, Spades)];

    // Игрок 2: 3♥, 7♥ → full house (3–3–3–K–K)
    let p2_hole = vec![c(Three, Hearts), c(Seven, Hearts)];

    let r1 = evaluate_best_hand(&p1_hole, &board);
    let r2 = evaluate_best_hand(&p2_hole, &board);

    assert!(r1 > r2, "Four of a kind должен быть сильнее full house");
}

//
// ============= ТЕСТ 3: flush > straight ============
//
#[test]
fn flush_beats_straight() {
    // Борд: 2♣, 4♣, 6♣, 8♦, T♦
    let board = vec![
        c(Two, Clubs),
        c(Four, Clubs),
        c(Six, Clubs),
        c(Eight, Diamonds),
        c(Ten, Diamonds),
    ];

    // Игрок 1: A♣, Q♣ → флеш по трефам
    let p1_hole = vec![c(Ace, Clubs), c(Queen, Clubs)];

    // Игрок 2: 5♦, 7♠ → стрит 4–8
    let p2_hole = vec![c(Five, Diamonds), c(Seven, Spades)];

    let r1 = evaluate_best_hand(&p1_hole, &board);
    let r2 = evaluate_best_hand(&p2_hole, &board);

    assert!(r1 > r2, "Flush должен быть сильнее straight");
}

//
// ============= ТЕСТ 4: split pot — одинаковая лучшая рука ============
//
#[test]
fn split_pot_same_best_hand() {
    // Борд даёт стрит 5–9 всем:
    // 5♣, 6♦, 7♥, 8♠, 9♣
    let board = vec![
        c(Five, Clubs),
        c(Six, Diamonds),
        c(Seven, Hearts),
        c(Eight, Spades),
        c(Nine, Clubs),
    ];

    // Игрок 1: A♣, A♦
    let p1_hole = vec![c(Ace, Clubs), c(Ace, Diamonds)];

    // Игрок 2: K♣, K♦
    let p2_hole = vec![c(King, Clubs), c(King, Diamonds)];

    // В обоих случаях лучшая рука — общий стрит 5–9 (борд),
    // поэтому HandRank должен быть одинаковый.
    let r1 = evaluate_best_hand(&p1_hole, &board);
    let r2 = evaluate_best_hand(&p2_hole, &board);

    assert_eq!(r1, r2, "Одинаковая лучшая рука → равный HandRank (split-pot ситуация)");
}

//
// ============= ТЕСТ 5: кикер решает (top pair, разные кикеры) ============
//
#[test]
fn kicker_decides_for_top_pair() {
    // Борд: K♣, 7♦, 3♠, 2♥, T♣
    let board = vec![
        c(King, Clubs),
        c(Seven, Diamonds),
        c(Three, Spades),
        c(Two, Hearts),
        c(Ten, Clubs),
    ];

    // Игрок 1: K♦, A♠ → top pair K + A kicker
    let p1_hole = vec![c(King, Diamonds), c(Ace, Spades)];

    // Игрок 2: K♥, Q♥ → top pair K + Q kicker
    let p2_hole = vec![c(King, Hearts), c(Queen, Hearts)];

    let r1 = evaluate_best_hand(&p1_hole, &board);
    let r2 = evaluate_best_hand(&p2_hole, &board);

    assert!(
        r1 > r2,
        "У игрока 1 кикер A, у игрока 2 — Q, A-кер должен выиграть"
    );
}
