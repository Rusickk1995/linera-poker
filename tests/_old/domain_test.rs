//! Интеграционные тесты для доменной модели (crate::domain).

use poker_engine::domain::*;

/// Тестируем AnteType и BlindLevel::new.
#[test]
fn blinds_level_new_and_fields() {
    let lvl = BlindLevel::new(
        3,
        Chips(50),
        Chips(100),
        Chips(10),
        AnteType::BigBlind,
        15,
    );

    assert_eq!(lvl.level, 3);
    assert_eq!(lvl.small_blind, Chips(50));
    assert_eq!(lvl.big_blind, Chips(100));
    assert_eq!(lvl.ante, Chips(10));
    assert_eq!(lvl.ante_type, AnteType::BigBlind);
    assert_eq!(lvl.duration_minutes, 15);
}

/// Тестируем BlindStructure: first_level, level_by_number, level_for_elapsed_minutes.
#[test]
fn blind_structure_level_selection() {
    let levels = vec![
        BlindLevel::new(1, Chips(25), Chips(50), Chips(0), AnteType::None, 10),
        BlindLevel::new(2, Chips(50), Chips(100), Chips(10), AnteType::Classic, 10),
        BlindLevel::new(3, Chips(75), Chips(150), Chips(25), AnteType::BigBlind, 20),
    ];
    let s = BlindStructure::new(levels);

    // first_level
    let first = s.first_level().unwrap();
    assert_eq!(first.level, 1);
    assert_eq!(first.small_blind, Chips(25));

    // level_by_number
    let lvl2 = s.level_by_number(2).unwrap();
    assert_eq!(lvl2.big_blind, Chips(100));

    // несуществующий номер — None
    assert!(s.level_by_number(999).is_none());

    // level_for_elapsed_minutes:
    // минуты считаем кумулятивно: [0..10) -> lvl1, [10..20) -> lvl2, [20..40) -> lvl3, дальше -> lvl3
    let l1 = s.level_for_elapsed_minutes(0).unwrap();
    assert_eq!(l1.level, 1);

    let l2 = s.level_for_elapsed_minutes(10).unwrap();
    assert_eq!(l2.level, 2);

    let l3 = s.level_for_elapsed_minutes(25).unwrap();
    assert_eq!(l3.level, 3);

    let l3_tail = s.level_for_elapsed_minutes(999).unwrap();
    assert_eq!(l3_tail.level, 3);
}

/// Card/Suit/Rank: Display + FromStr roundtrip.
#[test]
fn card_display_and_parse_roundtrip() {
    // несколько разных карт
    let cards = [
        Card::new(Rank::Ace, Suit::Hearts),      // Ah
        Card::new(Rank::Ten, Suit::Spades),      // Ts
        Card::new(Rank::Two, Suit::Clubs),       // 2c
        Card::new(Rank::Nine, Suit::Diamonds),   // 9d
    ];

    for card in cards {
        let s = card.to_string();
        let parsed: Card = s.parse().expect("parse Card from Display string");
        assert_eq!(parsed, card);
    }

    // Неверные строки
    assert!("".parse::<Card>().is_err());
    assert!("XYZ".parse::<Card>().is_err());
    assert!("1c".parse::<Card>().is_err());
    assert!("Acx".parse::<Card>().is_err());
}

/// Chips: арифметика и saturating_sub.
#[test]
fn chips_arithmetic_and_saturating() {
    let a = Chips(100);
    let b = Chips(50);
    let c = Chips(200);

    assert_eq!(a + b, Chips(150));
    assert_eq!(c - b, Chips(150));

    let mut x = Chips(10);
    x += Chips(5);
    assert_eq!(x, Chips(15));

    x -= Chips(20); // saturating_sub внутри
    assert_eq!(x, Chips(0));

    assert!(Chips::ZERO.is_zero());
    assert!(Chips(0).is_zero() && !Chips(1).is_zero());

    let d = Chips(10).saturating_sub(Chips(20));
    assert_eq!(d, Chips(0));
}

/// Deck: стандартная колода 52 карты, уникальные, draw_one/draw_n/remove_cards.
#[test]
fn deck_standard_52_basic_properties() {
    let deck = Deck::standard_52();
    assert_eq!(deck.len(), 52);
    assert!(!deck.is_empty());

    // Все карты должны быть уникальны.
    use std::collections::HashSet;
    let set: HashSet<_> = deck.cards.iter().collect();
    assert_eq!(set.len(), 52);

    // Проверим, что в каждой масти 13 карт.
    let mut clubs = 0;
    let mut diamonds = 0;
    let mut hearts = 0;
    let mut spades = 0;
    for c in &deck.cards {
        match c.suit {
            Suit::Clubs => clubs += 1,
            Suit::Diamonds => diamonds += 1,
            Suit::Hearts => hearts += 1,
            Suit::Spades => spades += 1,
        }
    }
    assert_eq!(clubs, 13);
    assert_eq!(diamonds, 13);
    assert_eq!(hearts, 13);
    assert_eq!(spades, 13);
}

#[test]
fn deck_draw_and_remove_cards() {
    let mut deck = Deck::standard_52();
    let original_len = deck.len();

    // draw_one
    let c1 = deck.draw_one().expect("should draw one");
    assert_eq!(deck.len(), original_len - 1);

    // draw_n больше, чем осталось
    let taken = deck.draw_n(60);
    assert_eq!(taken.len(), original_len - 1); // уже почти всё
    assert!(deck.is_empty());

    // remove_cards
    let mut deck2 = Deck::standard_52();
    let to_remove = [
        Card::new(Rank::Ace, Suit::Hearts),
        Card::new(Rank::King, Suit::Spades),
    ];
    deck2.remove_cards(&to_remove);
    assert_eq!(deck2.len(), 50);
    for card in &to_remove {
        assert!(!deck2.cards.contains(card));
    }
}

/// HandRank и PlayerHandResult/HandSummary – простые проверки структуры.
#[test]
fn hand_rank_and_summary_basic() {
    let r1 = HandRank(100);
    let r2 = HandRank(200);

    assert!(r2 > r1);
    assert_eq!(r1, HandRank(100));

    let player_res = PlayerHandResult {
        player_id: 42,
        rank: Some(r2),
        net_chips: Chips(300),
        is_winner: true,
    };

    assert_eq!(player_res.player_id, 42);
    assert_eq!(player_res.rank.unwrap(), r2);
    assert_eq!(player_res.net_chips, Chips(300));
    assert!(player_res.is_winner);

    let summary = HandSummary {
        hand_id: 1,
        table_id: 10,
        street_reached: Street::River,
        board: vec![
            Card::new(Rank::Ace, Suit::Spades),
            Card::new(Rank::King, Suit::Spades),
        ],
        total_pot: Chips(500),
        results: vec![player_res.clone()],
    };

    assert_eq!(summary.hand_id, 1);
    assert_eq!(summary.table_id, 10);
    assert_eq!(summary.street_reached, Street::River);
    assert_eq!(summary.total_pot, Chips(500));
    assert_eq!(summary.results.len(), 1);
    assert_eq!(summary.results[0], player_res);
}

/// PlayerAtTable::new и is_in_hand.
#[test]
fn player_at_table_new_and_is_in_hand() {
    let p = PlayerAtTable::new(123, Chips(1000));

    assert_eq!(p.player_id, 123);
    assert_eq!(p.stack, Chips(1000));
    assert_eq!(p.current_bet, Chips::ZERO);
    assert_eq!(p.status, PlayerStatus::Active);
    assert!(p.hole_cards.is_empty());
    assert!(p.is_in_hand());

    let mut p2 = p.clone();
    p2.status = PlayerStatus::Folded;
    assert!(!p2.is_in_hand());

    let mut p3 = p.clone();
    p3.status = PlayerStatus::AllIn;
    assert!(p3.is_in_hand());
}

/// Table::new, seated_count, is_seat_empty.
#[test]
fn table_new_and_seating_basic() {
    let stakes = TableStakes::new(
        Chips(50),
        Chips(100),
        AnteType::None,
        Chips::ZERO,
    );
    let cfg = TableConfig {
        max_seats: 9,
        table_type: TableType::Cash,
        stakes,
        allow_straddle: false,
        allow_run_it_twice: false,
    };

    let mut table = Table::new(1, "Test Table".to_string(), cfg);

    assert_eq!(table.id, 1);
    assert_eq!(table.name, "Test Table");
    assert_eq!(table.max_seats(), 9);
    assert_eq!(table.seated_count(), 0);
    assert_eq!(table.board.len(), 0);
    assert!(table.dealer_button.is_none());
    assert!(table.current_hand_id.is_none());
    assert_eq!(table.street, Street::Preflop);
    assert!(!table.hand_in_progress);
    assert_eq!(table.total_pot, Chips::ZERO);

    // все места пустые
    for i in 0..table.max_seats() {
        assert!(table.is_seat_empty(i));
    }

    // посадим игрока на SeatIndex 0
    table.seats[0] = Some(PlayerAtTable::new(1, Chips(500)));
    assert_eq!(table.seated_count(), 1);
    assert!(!table.is_seat_empty(0));
}

/// Tournament::new и базовые поля.
#[test]
fn tournament_new_and_defaults() {
    let levels = vec![
        BlindLevel::new(1, Chips(25), Chips(50), Chips(0), AnteType::None, 10),
    ];
    let blinds = BlindStructure::new(levels);

    let cfg = TournamentConfig {
        starting_stack: Chips(10000),
        max_players: Some(100),
        table_size: 9,
        blinds,
        is_freezeout: true,
        reentry_allowed: false,
        max_reentries: None,
    };

    let t = Tournament::new(7, "Sunday Special".to_string(), cfg);

    assert_eq!(t.id, 7);
    assert_eq!(t.name, "Sunday Special");
    assert_eq!(t.status, TournamentStatus::Registering);
    assert_eq!(t.current_level, 1);
    assert_eq!(t.hands_played, 0);
    assert!(t.players.is_empty());
    assert!(t.tables.is_empty());

    assert_eq!(t.config.starting_stack, Chips(10000));
    assert_eq!(t.config.max_players, Some(100));
    assert_eq!(t.config.table_size, 9);
    assert!(t.config.is_freezeout);
    assert!(!t.config.reentry_allowed);
}

/// TournamentPlayer структура: базовая инициализация.
#[test]
fn tournament_player_struct_basic() {
    let p = TournamentPlayer {
        player_id: 42,
        total_chips: Chips(5000),
        is_busted: false,
        table_id: Some(1),
        seat_index: Some(0),
        finishing_place: None,
    };

    assert_eq!(p.player_id, 42);
    assert_eq!(p.total_chips, Chips(5000));
    assert!(!p.is_busted);
    assert_eq!(p.table_id, Some(1));
    assert_eq!(p.seat_index, Some(0));
    assert_eq!(p.finishing_place, None);
}
