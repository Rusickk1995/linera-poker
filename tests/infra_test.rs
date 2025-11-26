// tests/infra_test.rs

use std::collections::HashMap;

use poker_engine::{
    api::AnteTypeApi,
    domain::{
        blinds::{AnteType, BlindLevel, BlindStructure},
        chips::Chips,
        hand::Street,
        player::{PlayerAtTable, PlayerStatus},
        table::{Table, TableConfig, TableStakes, TableType},
        tournament::{Tournament, TournamentConfig, TournamentStatus, TournamentPlayer},
        Card, Rank, Suit, TableId, PlayerId,
    },
    engine::{
        HandEngine,
        HandHistory,
        Pot,
        betting::BettingState,
        RandomSource, // ВАЖНО: этот trait даёт нам метод shuffle()
    },
    infra::{
        ids::{ExternalId, IdGenerator},
        mapping::{
            ante_type_from_api,
            ante_type_to_api,
            DefaultNameResolver,
            PlayerNameResolver,
            is_seat_active,
            map_table_to_dto,
        },
        persistence::{InMemoryPokerStorage, PokerStorage},
        rng::{DeterministicRng, SystemRng},
    },
};

//
// ---------- helpers ----------
//

fn make_table_basic(id: TableId) -> Table {
    let stakes = TableStakes::new(
        Chips::new(50),
        Chips::new(100),
        AnteType::None,
        Chips::ZERO,
    );

    let config = TableConfig {
        max_seats: 6,
        table_type: TableType::Cash,
        stakes,
        allow_straddle: false,
        allow_run_it_twice: false,
    };

    Table::new(id, "Test Table".to_string(), config)
}

fn seat_player(
    table: &mut Table,
    seat_index: u8,
    player_id: PlayerId,
    stack: u64,
    status: PlayerStatus,
) {
    let pat = PlayerAtTable {
        player_id,
        stack: Chips::new(stack),
        current_bet: Chips::ZERO,
        status,
        hole_cards: vec![],
    };
    table.seats[seat_index as usize] = Some(pat);
}

fn make_dummy_hand_engine(table_id: TableId, current_actor_seat: Option<u8>) -> HandEngine {
    use poker_engine::domain::deck::Deck;

    let deck = Deck::standard_52();
    let betting = BettingState::new(Street::Preflop, Chips::ZERO, Chips::new(100), vec![]);
    let pot = Pot::new();
    let side_pots = Vec::new();
    let contributions = HashMap::new();
    let history = HandHistory::new();

    HandEngine {
        table_id,
        hand_id: 1,
        deck,
        betting,
        pot,
        side_pots,
        contributions,
        current_actor: current_actor_seat.map(|s| s as u8),
        history,
    }
}

//
// ---------- ids.rs tests ----------
//

#[test]
fn id_generator_produces_sequential_ids() {
    let gen = IdGenerator::new();

    let t1 = gen.next_table_id();
    let t2 = gen.next_table_id();
    assert_eq!(t2, t1 + 1);

    let p1 = gen.next_player_id();
    let p2 = gen.next_player_id();
    assert_eq!(p2, p1 + 1);

    let tour1 = gen.next_tournament_id();
    let tour2 = gen.next_tournament_id();
    assert_eq!(tour2, tour1 + 1);

    let h1 = gen.next_hand_id();
    let h2 = gen.next_hand_id();
    assert_eq!(h2, h1 + 1);

    // Независимость разных генераторов
    let gen2 = IdGenerator::new();
    let t_first = gen2.next_table_id();
    let p_first = gen2.next_player_id();
    assert_eq!(t_first, 1);
    assert_eq!(p_first, 1);
}

#[test]
fn external_id_wraps_string_and_compares_by_value() {
    let e1 = ExternalId("abc".to_string());
    let e2 = ExternalId("abc".to_string());
    let e3 = ExternalId("xyz".to_string());

    assert_eq!(e1, e2);
    assert_ne!(e1, e3);
}

//
// ---------- mapping.rs tests ----------
//

#[test]
fn ante_type_roundtrip_between_api_and_domain() {
    // domain -> api (через pattern matching, без PartialEq)
    match ante_type_to_api(AnteType::None) {
        AnteTypeApi::None => {}
        _ => panic!("expected AnteTypeApi::None"),
    }
    match ante_type_to_api(AnteType::Classic) {
        AnteTypeApi::Classic => {}
        _ => panic!("expected AnteTypeApi::Classic"),
    }
    match ante_type_to_api(AnteType::BigBlind) {
        AnteTypeApi::BigBlind => {}
        _ => panic!("expected AnteTypeApi::BigBlind"),
    }

    // api -> domain (AnteType имеет PartialEq, тут assert_eq нормально)
    assert_eq!(ante_type_from_api(AnteTypeApi::None), AnteType::None);
    assert_eq!(ante_type_from_api(AnteTypeApi::Classic), AnteType::Classic);
    assert_eq!(ante_type_from_api(AnteTypeApi::BigBlind), AnteType::BigBlind);
}

#[test]
fn default_name_resolver_formats_player_name() {
    let resolver = DefaultNameResolver;
    let name = resolver.resolve_name(42);
    assert_eq!(name, "Player 42");
}

#[test]
fn map_table_to_dto_basic_mapping_without_engine() {
    let mut table = make_table_basic(100);
    table.street = Street::Flop;
    table.dealer_button = Some(2);
    table.total_pot = Chips::new(1234);
    table.board = vec![
        Card { rank: Rank::Ace, suit: Suit::Spades },
        Card { rank: Rank::King, suit: Suit::Spades },
        Card { rank: Rank::Queen, suit: Suit::Spades },
    ];
    table.hand_in_progress = true;

    // Два игрока: hero и оппонент
    seat_player(&mut table, 0, 1, 5000, PlayerStatus::Active);
    seat_player(&mut table, 1, 2, 4000, PlayerStatus::Active);

    if let Some(p) = table.seats[0].as_mut() {
        p.hole_cards = vec![
            Card { rank: Rank::Ace, suit: Suit::Hearts },
            Card { rank: Rank::Ace, suit: Suit::Diamonds },
        ];
    }
    if let Some(p) = table.seats[1].as_mut() {
        p.hole_cards = vec![
            Card { rank: Rank::Two, suit: Suit::Clubs },
            Card { rank: Rank::Three, suit: Suit::Clubs },
        ];
    }

    let resolver = DefaultNameResolver;
    let is_hero = |pid: PlayerId| pid == 1;

    let dto = map_table_to_dto(&table, None, &resolver, is_hero);

    assert_eq!(dto.table_id, table.id);
    assert_eq!(dto.name, table.name);
    assert_eq!(dto.max_seats, table.config.max_seats);
    assert_eq!(dto.small_blind, table.config.stakes.small_blind);
    assert_eq!(dto.big_blind, table.config.stakes.big_blind);
    assert_eq!(dto.ante, table.config.stakes.ante);
    assert_eq!(dto.street, table.street);
    assert_eq!(dto.dealer_button, table.dealer_button.map(|s| s as u8));
    assert_eq!(dto.total_pot, table.total_pot);
    assert_eq!(dto.board, table.board);
    assert!(dto.hand_in_progress);
    assert_eq!(dto.current_actor_seat, None);

    assert_eq!(dto.players.len(), 2);

    let p0 = &dto.players[0];
    assert_eq!(p0.player_id, 1);
    assert_eq!(p0.display_name, "Player 1");
    assert_eq!(p0.seat_index, 0);
    assert!(p0.hole_cards.is_some());

    let p1 = &dto.players[1];
    assert_eq!(p1.player_id, 2);
    assert_eq!(p1.display_name, "Player 2");
    assert_eq!(p1.seat_index, 1);
    assert!(p1.hole_cards.is_none());
}

#[test]
fn map_table_to_dto_includes_current_actor_from_engine() {
    let mut table = make_table_basic(200);
    seat_player(&mut table, 0, 10, 1000, PlayerStatus::Active);
    seat_player(&mut table, 1, 20, 1000, PlayerStatus::Active);

    let resolver = DefaultNameResolver;
    let engine = make_dummy_hand_engine(table.id, Some(1)); // ход игрока seat=1

    let dto = map_table_to_dto(&table, Some(&engine), &resolver, |_| false);

    assert_eq!(dto.current_actor_seat, Some(1u8));
}

#[test]
fn is_seat_active_checks_status_correctly() {
    let mut table = make_table_basic(300);

    seat_player(&mut table, 0, 1, 1000, PlayerStatus::Active);
    seat_player(&mut table, 1, 2, 0, PlayerStatus::AllIn);
    seat_player(&mut table, 2, 3, 900, PlayerStatus::Folded);
    seat_player(&mut table, 3, 4, 1000, PlayerStatus::SittingOut);
    seat_player(&mut table, 4, 5, 0, PlayerStatus::Busted);

    assert!(is_seat_active(&table, 0));
    assert!(is_seat_active(&table, 1));
    assert!(!is_seat_active(&table, 2));
    assert!(!is_seat_active(&table, 3));
    assert!(!is_seat_active(&table, 4));
    assert!(!is_seat_active(&table, 5));
    assert!(!is_seat_active(&table, 999));
}

//
// ---------- persistence.rs tests ----------
//

#[test]
fn in_memory_storage_saves_and_loads_tables() {
    let mut storage = InMemoryPokerStorage::new();

    let mut table = make_table_basic(1);
    seat_player(&mut table, 0, 10, 1500, PlayerStatus::Active);

    storage.save_table(&table);

    let loaded = storage.load_table(1).expect("table should exist");
    assert_eq!(loaded.id, table.id);
    assert_eq!(loaded.name, table.name);
    assert_eq!(loaded.config.max_seats, table.config.max_seats);

    assert!(storage.load_table(999).is_none());
}

#[test]
fn in_memory_storage_saves_and_loads_tournaments() {
    let blinds = BlindStructure::new(vec![BlindLevel::new(
        1,
        Chips::new(50),
        Chips::new(100),
        Chips::ZERO,
        AnteType::None,
        10,
    )]);

    let config = TournamentConfig {
        starting_stack: Chips::new(10000),
        max_players: Some(100),
        table_size: 9,
        blinds,
        is_freezeout: true,
        reentry_allowed: false,
        max_reentries: None,
    };

    let mut tournament = Tournament::new(5, "Main Event".to_string(), config);
    tournament.status = TournamentStatus::Running;
    tournament.players.push(TournamentPlayer {
        player_id: 1,
        total_chips: Chips::new(10000),
        is_busted: false,
        table_id: Some(1),
        seat_index: Some(0),
        finishing_place: None,
    });

    let mut storage = InMemoryPokerStorage::new();
    storage.save_tournament(&tournament);

    let loaded = storage
        .load_tournament(5)
        .expect("tournament should exist");

    assert_eq!(loaded.id, tournament.id);
    assert_eq!(loaded.name, tournament.name);
    assert_eq!(loaded.status, TournamentStatus::Running);
    assert_eq!(loaded.players.len(), 1);
    assert_eq!(loaded.players[0].player_id, 1);

    assert!(storage.load_tournament(9999).is_none());
}

#[test]
fn in_memory_storage_handles_active_hand_none() {
    let mut storage = InMemoryPokerStorage::new();
    let table_id: TableId = 77;

    assert!(storage.load_active_hand(table_id).is_none());

    storage.save_active_hand(table_id, None);
    assert!(storage.load_active_hand(table_id).is_none());
}

//
// ---------- rng.rs tests ----------
//

#[test]
fn system_rng_shuffle_produces_permutation() {
    let mut rng = SystemRng::default();
    let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];

    rng.shuffle(&mut data);

    let mut sorted = data.clone();
    sorted.sort();
    assert_eq!(sorted, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn deterministic_rng_produces_repeatable_shuffle() {
    let mut r1 = DeterministicRng::from_seed(42);
    let mut r2 = DeterministicRng::from_seed(42);

    let mut a1 = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut a2 = vec![1, 2, 3, 4, 5, 6, 7, 8];

    r1.shuffle(&mut a1);
    r2.shuffle(&mut a2);

    assert_eq!(a1, a2);
}
