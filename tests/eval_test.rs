// tests/eval_test.rs

use poker_engine::{
    domain::{
        card::{Card, Rank, Suit},
        hand::HandRank,
    },
    eval::{
        describe_hand,
        evaluate_best_hand,
        hand_category,
        HandCategory,
    },
};

use poker_engine::eval::lookup_tables::{
    RankMask,
    STRAIGHT_MASKS,
    detect_straight,
    mask_from_ranks,
    rank_to_bit,
};

/// Утилита: удобный конструктор карты.
fn c(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit)
}

//
// ---- Тесты для lookup_tables ----
//

#[test]
fn rank_to_bit_basic() {
    let two_bit = rank_to_bit(Rank::Two);
    let ace_bit = rank_to_bit(Rank::Ace);

    // Rank::Two → младший бит.
    assert_eq!(two_bit, 1u16 << 0);
    // Rank::Ace → старший из 13 бит (2..A).
    assert_eq!(ace_bit, 1u16 << 12);
}

#[test]
fn mask_from_ranks_builds_correct_mask() {
    let mask: RankMask = mask_from_ranks(&[Rank::Two, Rank::Four, Rank::Ace]);

    let expected = rank_to_bit(Rank::Two)
        | rank_to_bit(Rank::Four)
        | rank_to_bit(Rank::Ace);

    assert_eq!(mask, expected);
}

#[test]
fn detect_straight_wheel_and_broadway() {
    // wheel A2345 – должен вернуть Rank::Five
    let wheel_mask = STRAIGHT_MASKS[0];
    let detected_wheel = detect_straight(wheel_mask);
    assert_eq!(detected_wheel, Some(Rank::Five));

    // broadway TJQKA – должен вернуть Rank::Ace
    let broadway_mask = STRAIGHT_MASKS[9];
    let detected_broadway = detect_straight(broadway_mask);
    assert_eq!(detected_broadway, Some(Rank::Ace));
}

#[test]
fn detect_straight_none_when_gap() {
    // Маска без 5 подряд
    let mask = rank_to_bit(Rank::Two)
        | rank_to_bit(Rank::Four)
        | rank_to_bit(Rank::Seven)
        | rank_to_bit(Rank::Ace);

    assert_eq!(detect_straight(mask), None);
}

//
// ---- Тесты для hand_rank ----
//

#[test]
fn hand_rank_encoding_roundtrip() {
    let category = HandCategory::FullHouse;
    let ranks = [
        Rank::Ace,
        Rank::Ace,
        Rank::Ace,
        Rank::King,
        Rank::King,
    ];

    let hr = HandRank::from_category_and_ranks(category, ranks);
    assert_eq!(hr.category(), category);

    let decoded = hr.ranks();
    assert_eq!(decoded, ranks);
}

#[test]
fn hand_rank_ordering_by_category_and_ranks() {
    // Флеш слабее фулл-хауса
    let flush = HandRank::from_category_and_ranks(
        HandCategory::Flush,
        [Rank::Ace, Rank::King, Rank::Queen, Rank::Jack, Rank::Ten],
    );
    let full_house = HandRank::from_category_and_ranks(
        HandCategory::FullHouse,
        [Rank::Queen, Rank::Queen, Rank::Queen, Rank::Two, Rank::Two],
    );
    assert!(full_house > flush);

    // Straight A-high сильнее Straight K-high
    let broadway = HandRank::from_category_and_ranks(
        HandCategory::Straight,
        [Rank::Ace, Rank::King, Rank::Queen, Rank::Jack, Rank::Ten],
    );
    let king_high = HandRank::from_category_and_ranks(
        HandCategory::Straight,
        [Rank::King, Rank::Queen, Rank::Jack, Rank::Ten, Rank::Nine],
    );
    assert!(broadway > king_high);
}

#[test]
fn hand_category_and_description_match() {
    let hr = HandRank::from_category_and_ranks(
        HandCategory::ThreeOfAKind,
        [Rank::Ten, Rank::Ten, Rank::Ten, Rank::Five, Rank::Three],
    );

    assert_eq!(hand_category(hr), HandCategory::ThreeOfAKind);
    assert_eq!(describe_hand(hr), "Three of a kind".to_string());
}

//
// ---- Тесты для evaluator::evaluate_best_hand ----
//

#[test]
fn evaluate_best_hand_high_card() {
    // Ah 7d / board: 2c 9s Jd 3h Kc
    let hole = [c(Rank::Ace, Suit::Hearts), c(Rank::Seven, Suit::Diamonds)];
    let board = [
        c(Rank::Two, Suit::Clubs),
        c(Rank::Nine, Suit::Spades),
        c(Rank::Jack, Suit::Diamonds),
        c(Rank::Three, Suit::Hearts),
        c(Rank::King, Suit::Clubs),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::HighCard);

    let ranks = hr.ranks();
    // Топ — туз, затем король и т.д. (конкретный порядок не критичен, но туз должен быть старшим)
    assert_eq!(ranks[0], Rank::Ace);
}

#[test]
fn evaluate_best_hand_one_pair() {
    // Ah As / board: 2c 9s Jd 3h Kc → пара тузов
    let hole = [c(Rank::Ace, Suit::Hearts), c(Rank::Ace, Suit::Spades)];
    let board = [
        c(Rank::Two, Suit::Clubs),
        c(Rank::Nine, Suit::Spades),
        c(Rank::Jack, Suit::Diamonds),
        c(Rank::Three, Suit::Hearts),
        c(Rank::King, Suit::Clubs),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::OnePair);
    let ranks = hr.ranks();
    assert_eq!(ranks[0], Rank::Ace); // пара тузов как старшая компонента
}

#[test]
fn evaluate_best_hand_two_pair() {
    // Ah As / board: Kc Kh 2d 3c 7s → две пары A,K
    let hole = [c(Rank::Ace, Suit::Hearts), c(Rank::Ace, Suit::Spades)];
    let board = [
        c(Rank::King, Suit::Clubs),
        c(Rank::King, Suit::Hearts),
        c(Rank::Two, Suit::Diamonds),
        c(Rank::Three, Suit::Clubs),
        c(Rank::Seven, Suit::Spades),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::TwoPair);
}

#[test]
fn evaluate_best_hand_straight() {
    // 7h 8d / board: 5s 6c 9h Kd 2c → стрит 5-9
    let hole = [c(Rank::Seven, Suit::Hearts), c(Rank::Eight, Suit::Diamonds)];
    let board = [
        c(Rank::Five, Suit::Spades),
        c(Rank::Six, Suit::Clubs),
        c(Rank::Nine, Suit::Hearts),
        c(Rank::King, Suit::Diamonds),
        c(Rank::Two, Suit::Clubs),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::Straight);

    let ranks = hr.ranks();
    // Старшая карта стрита должна быть Nine
    assert_eq!(ranks[0], Rank::Nine);
}

#[test]
fn evaluate_best_hand_flush() {
    // Ah 2h / board: 5h 9h Kh 3c 7d → флеш по хартам
    let hole = [c(Rank::Ace, Suit::Hearts), c(Rank::Two, Suit::Hearts)];
    let board = [
        c(Rank::Five, Suit::Hearts),
        c(Rank::Nine, Suit::Hearts),
        c(Rank::King, Suit::Hearts),
        c(Rank::Three, Suit::Clubs),
        c(Rank::Seven, Suit::Diamonds),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::Flush);

    let ranks = hr.ranks();
    assert_eq!(ranks[0], Rank::Ace);
    assert_eq!(ranks[1], Rank::King);
}

#[test]
fn evaluate_best_hand_full_house_from_seven_cards() {
    // Ah Ad / board: As Kc Kh 2d 3c → лучшие 5: AAAKK (full house)
    let hole = [c(Rank::Ace, Suit::Hearts), c(Rank::Ace, Suit::Diamonds)];
    let board = [
        c(Rank::Ace, Suit::Spades),
        c(Rank::King, Suit::Clubs),
        c(Rank::King, Suit::Hearts),
        c(Rank::Two, Suit::Diamonds),
        c(Rank::Three, Suit::Clubs),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::FullHouse);

    let ranks = hr.ranks();
    // trips = A, pair = K (см. код full house в evaluator)
    assert_eq!(ranks[0], Rank::Ace);
    assert_eq!(ranks[1], Rank::King);
}

#[test]
fn evaluate_best_hand_four_of_a_kind() {
    // Ah Ad / board: As Ac Kc 2d 3c → каре тузов
    let hole = [c(Rank::Ace, Suit::Hearts), c(Rank::Ace, Suit::Diamonds)];
    let board = [
        c(Rank::Ace, Suit::Spades),
        c(Rank::Ace, Suit::Clubs),
        c(Rank::King, Suit::Clubs),
        c(Rank::Two, Suit::Diamonds),
        c(Rank::Three, Suit::Clubs),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::FourOfAKind);

    let ranks = hr.ranks();
    assert_eq!(ranks[0], Rank::Ace);
}

#[test]
fn evaluate_best_hand_straight_flush() {
    // 9h Th / board: Jh Qh Kh 2c 3d → стрит-флеш K-high
    let hole = [c(Rank::Nine, Suit::Hearts), c(Rank::Ten, Suit::Hearts)];
    let board = [
        c(Rank::Jack, Suit::Hearts),
        c(Rank::Queen, Suit::Hearts),
        c(Rank::King, Suit::Hearts),
        c(Rank::Two, Suit::Clubs),
        c(Rank::Three, Suit::Diamonds),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::StraightFlush);

    let ranks = hr.ranks();
    // Старший должен быть король (9-T-J-Q-K).
    assert_eq!(ranks[0], Rank::King);
}

#[test]
fn evaluate_best_hand_prefers_stronger_category_from_seven_cards() {
    // Конфликтующая ситуация: на доске уже есть стрит,
    // но hole даёт full house → должна выбрать full house.
    //
    // board: 5c 6d 7h 8s 9c  (стрит 5-9)
    // hole:  Kd Kh          (пара королей, но с доской можно сделать фуллхаус  KKK77? нет)
    //
    // Давай сделаем так:
    // board: Kc Kd 7h 8s 9c
    // hole:  Kh Ks → каре королей против возможных других комбинаций.
    let hole = [c(Rank::King, Suit::Hearts), c(Rank::King, Suit::Spades)];
    let board = [
        c(Rank::King, Suit::Clubs),
        c(Rank::King, Suit::Diamonds),
        c(Rank::Seven, Suit::Hearts),
        c(Rank::Eight, Suit::Spades),
        c(Rank::Nine, Suit::Clubs),
    ];

    let hr = evaluate_best_hand(&hole, &board);
    assert_eq!(hand_category(hr), HandCategory::FourOfAKind);
}
