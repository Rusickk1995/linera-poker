use poker_engine::domain::{
    blinds::AnteType,
    card::{Card, Rank, Suit},
    chips::Chips,
    deck::Deck,
    hand::Street,
    player::{PlayerAtTable, PlayerStatus},
    table::{SeatIndex, Table, TableConfig, TableStakes, TableType},
};
use poker_engine::engine::{
    self,
    actions::{PlayerAction, PlayerActionKind},
    betting::BettingState,
    errors::EngineError,
    game_loop::{apply_action, start_hand, HandStatus},
    hand_history::{HandEventKind, HandHistory},
    positions::{collect_occupied_seats_from, next_dealer, next_occupied_seat},
    pot::Pot,
    side_pots::{compute_side_pots, SidePot},
    validation::validate_action,
    RandomSource,
};

/// Простой детерминированный RNG для тестов:
/// shuffle ничего не делает => колода остаётся в стандартном порядке.
#[derive(Default)]
struct DummyRng;

impl RandomSource for DummyRng {
    fn shuffle<T>(&mut self, _slice: &mut [T]) {
        // no-op
    }
}

fn make_heads_up_table() -> Table {
    let config = TableConfig {
        max_seats: 2,
        table_type: TableType::Cash,
        stakes: TableStakes::new(
            Chips(50),          // SB
            Chips(100),         // BB
            AnteType::None,     // без анте
            Chips::ZERO,
        ),
        allow_straddle: false,
        allow_run_it_twice: false,
    };

    let mut table = Table::new(1, "HU".to_string(), config);
    table.seats[0] = Some(PlayerAtTable::new(1, Chips(10_000)));
    table.seats[1] = Some(PlayerAtTable::new(2, Chips(10_000)));
    table
}

//
// actions.rs
//
#[test]
fn actions_basic_construction_and_equality() {
    let a1 = PlayerAction {
        player_id: 1,
        seat: 0,
        kind: PlayerActionKind::Call,
    };
    let a2 = PlayerAction {
        player_id: 1,
        seat: 0,
        kind: PlayerActionKind::Call,
    };
    let a3 = PlayerAction {
        player_id: 1,
        seat: 0,
        kind: PlayerActionKind::Raise(Chips(500)),
    };

    assert_eq!(a1, a2);
    assert_ne!(a1, a3);

    if let PlayerActionKind::Raise(size) = a3.kind {
        assert_eq!(size, Chips(500));
    } else {
        panic!("expected Raise");
    }
}

//
// betting.rs
//
#[test]
fn betting_state_mark_acted_and_round_complete() {
    let mut bs = BettingState::new(
        Street::Preflop,
        Chips(100),
        Chips(100),
        vec![0, 1, 2],
    );

    assert!(!bs.is_round_complete());
    bs.mark_acted(1);
    assert_eq!(bs.to_act, vec![0, 2]);

    bs.mark_acted(0);
    assert_eq!(bs.to_act, vec![2]);

    bs.mark_acted(2);
    assert!(bs.is_round_complete());
}

#[test]
fn betting_state_on_raise_updates_state() {
    let mut bs = BettingState::new(
        Street::Flop,
        Chips(100),
        Chips(100),
        vec![1, 2],
    );

    bs.on_raise(1, Chips(300), Chips(200), vec![2]);

    assert_eq!(bs.current_bet, Chips(300));
    assert_eq!(bs.min_raise, Chips(200));
    assert_eq!(bs.last_aggressor, Some(1));
    assert_eq!(bs.to_act, vec![2]);
}

//
// hand_history.rs
//
#[test]
fn hand_history_push_assigns_incremental_indices() {
    let mut h = HandHistory::new();
    assert!(h.events.is_empty());

    h.push(HandEventKind::HandStarted {
        table_id: 1,
        hand_id: 42,
    });

    h.push(HandEventKind::StreetChanged {
        street: Street::Flop,
    });

    assert_eq!(h.events.len(), 2);
    assert_eq!(h.events[0].index, 0);
    assert_eq!(h.events[1].index, 1);

    match &h.events[1].kind {
        HandEventKind::StreetChanged { street } => {
            assert_eq!(*street, Street::Flop);
        }
        _ => panic!("expected StreetChanged"),
    }
}

//
// pot.rs
//
#[test]
fn pot_add_and_reset_works() {
    let mut pot = Pot::new();
    assert_eq!(pot.total, Chips::ZERO);

    pot.add(Chips(100));
    pot.add(Chips(200));
    assert_eq!(pot.total, Chips(300));

    pot.reset();
    assert_eq!(pot.total, Chips::ZERO);
}

//
// side_pots.rs
//
#[test]
fn side_pots_single_pot_when_equal_contributions() {
    use std::collections::HashMap;

    let mut contribs = HashMap::new();
    contribs.insert(0u8, Chips(1000));
    contribs.insert(1u8, Chips(1000));
    contribs.insert(2u8, Chips(1000));

    let pots = compute_side_pots(&contribs);
    assert_eq!(pots.len(), 1);

    let p = &pots[0];
    assert_eq!(p.amount, Chips(3000));
    assert_eq!(p.eligible_seats.len(), 3);
}

#[test]
fn side_pots_all_in_creates_main_and_side_pots_with_three_levels() {
    use std::collections::HashMap;

    // P0: 1000, P1: 2000, P2: 4000
    let mut contribs = HashMap::new();
    contribs.insert(0u8, Chips(1000));
    contribs.insert(1u8, Chips(2000));
    contribs.insert(2u8, Chips(4000));

    let pots: Vec<SidePot> = compute_side_pots(&contribs);

    // Твой текущий алгоритм делает 3 пота:
    // 1) 1000 * 3 = 3000, eligible = {0,1,2}
    // 2) (2000-1000) * 2 = 2000, eligible = {1,2}
    // 3) (4000-2000) * 1 = 2000, eligible = {2}
    assert_eq!(pots.len(), 3);

    assert_eq!(pots[0].amount, Chips(3000));
    assert_eq!(pots[0].eligible_seats.len(), 3);
    assert!(pots[0].eligible_seats.contains(&0));
    assert!(pots[0].eligible_seats.contains(&1));
    assert!(pots[0].eligible_seats.contains(&2));

    assert_eq!(pots[1].amount, Chips(2000));
    assert_eq!(pots[1].eligible_seats.len(), 2);
    assert!(pots[1].eligible_seats.contains(&1));
    assert!(pots[1].eligible_seats.contains(&2));

    assert_eq!(pots[2].amount, Chips(2000));
    assert_eq!(pots[2].eligible_seats.len(), 1);
    assert!(pots[2].eligible_seats.contains(&2));
}


//
// positions.rs
//
#[test]
fn positions_next_occupied_and_collect_and_dealer() {
    // Стол на 6 мест, заняты 1,3,4
    let config = TableConfig {
        max_seats: 6,
        table_type: TableType::Cash,
        stakes: TableStakes::new(
            Chips(50),
            Chips(100),
            AnteType::None,
            Chips::ZERO,
        ),
        allow_straddle: false,
        allow_run_it_twice: false,
    };
    let mut table = Table::new(1, "Positions".into(), config);
    table.seats[1] = Some(PlayerAtTable::new(1, Chips(1000)));
    table.seats[3] = Some(PlayerAtTable::new(2, Chips(1000)));
    table.seats[4] = Some(PlayerAtTable::new(3, Chips(1000)));

    // next_occupied_seat
    let n0 = next_occupied_seat(&table, 0, true).unwrap();
    assert_eq!(n0, 1);

    let n1 = next_occupied_seat(&table, 1, false).unwrap();
    assert_eq!(n1, 3);

    // collect_occupied_seats_from
    let from1 = collect_occupied_seats_from(&table, 1);
    assert_eq!(from1, vec![1, 3, 4]);

    let from3 = collect_occupied_seats_from(&table, 3);
    assert_eq!(from3, vec![3, 4, 1]);

    // next_dealer: если кнопки нет – первый занятый с 0.
    assert_eq!(table.dealer_button, None);
    let d = next_dealer(&table).unwrap();
    assert_eq!(d, 1);

    // Теперь кнопка на 3 → следующий дилер 4.
    table.dealer_button = Some(3);
    let d2 = next_dealer(&table).unwrap();
    assert_eq!(d2, 4);
}

//
// validation.rs
//
fn make_player(stack: u64, current_bet: u64) -> PlayerAtTable {
    PlayerAtTable {
        player_id: 1,
        stack: Chips(stack),
        current_bet: Chips(current_bet),
        status: PlayerStatus::Active,
        hole_cards: Vec::new(),
    }
}

fn make_betting(current_bet: u64, min_raise: u64) -> BettingState {
    BettingState::new(
        Street::Flop,
        Chips(current_bet),
        Chips(min_raise),
        vec![],
    )
}

#[test]
fn validate_check_ok_when_no_bet() {
    let p = make_player(1000, 0);
    let b = make_betting(0, 100);
    validate_action(&p, &PlayerActionKind::Check, &b).unwrap();
}

#[test]
fn validate_check_fails_when_bet_exists() {
    let p = make_player(1000, 0);
    let b = make_betting(100, 100);
    let err = validate_action(&p, &PlayerActionKind::Check, &b).unwrap_err();
    assert!(matches!(err, EngineError::CannotCheck));
}

#[test]
fn validate_call_ok_and_cannot_call_when_no_bet() {
    let p = make_player(1000, 0);
    let b = make_betting(100, 100);
    validate_action(&p, &PlayerActionKind::Call, &b).unwrap();

    let b2 = make_betting(0, 100);
    let err = validate_action(&p, &PlayerActionKind::Call, &b2).unwrap_err();
    assert!(matches!(err, EngineError::CannotCall));
}

#[test]
fn validate_bet_and_raise_and_all_in_rules() {
    let p = make_player(1000, 0);

    // Bet когда нет ставки — ок, если сумма > 0 и стек >= bet
    let b0 = make_betting(0, 100);
    validate_action(&p, &PlayerActionKind::Bet(Chips(200)), &b0).unwrap();

    // Bet при уже существующей ставке — нельзя
    let b1 = make_betting(100, 100);
    let err = validate_action(&p, &PlayerActionKind::Bet(Chips(200)), &b1).unwrap_err();
    assert!(matches!(err, EngineError::IllegalAction));

    // Raise: ставка есть, raise не меньше min_raise
    let mut p2 = make_player(1000, 100); // уже заколлировал 100
    let b2 = make_betting(100, 100);
    validate_action(&p2, &PlayerActionKind::Raise(Chips(300)), &b2).unwrap();

    // Raise слишком маленький
    let err = validate_action(&p2, &PlayerActionKind::Raise(Chips(150)), &b2).unwrap_err();
    assert!(matches!(err, EngineError::RaiseTooSmall));

    // All-in нельзя, если стек 0
    let mut p3 = make_player(0, 0);
    let b3 = make_betting(0, 100);
    let err = validate_action(&p3, &PlayerActionKind::AllIn, &b3).unwrap_err();
    assert!(matches!(err, EngineError::IllegalAction));

    // All-in можно, если есть стек
    let p4 = make_player(500, 0);
    validate_action(&p4, &PlayerActionKind::AllIn, &b3).unwrap();
}

//
// game_loop.rs – базовые smoke-тесты
//
#[test]
fn start_hand_initializes_state_and_posts_blinds() {
    let mut table = make_heads_up_table();
    let mut rng = DummyRng::default();

    let engine = start_hand(&mut table, &mut rng, 1).expect("start_hand failed");

    // hand_in_progress и street
    assert!(table.hand_in_progress);
    assert_eq!(table.street, Street::Preflop);
    assert_eq!(table.board.len(), 0);

    // дилер установлен
    let dealer = table.dealer_button.expect("dealer not set");
    // В heads-up при наших настройках дилер должен быть на одном из занятых мест
    assert!(dealer == 0 || dealer == 1);

    // у каждого игрока по 2 карты
    for seat in 0..2usize {
        let p = table.seats[seat].as_ref().unwrap();
        assert_eq!(p.hole_cards.len(), 2);
    }

    // pot = SB + BB (без анте)
    let stakes = table.config.stakes.clone();
    assert_eq!(engine.pot.total, stakes.small_blind + stakes.big_blind);

    // current_bet = BB
    assert_eq!(engine.betting.current_bet, stakes.big_blind);
    assert!(engine.current_actor.is_some());
}

#[test]
fn apply_action_fold_finishes_hand_heads_up() {
    let mut table = make_heads_up_table();
    let mut rng = DummyRng::default();
    let mut engine = start_hand(&mut table, &mut rng, 1).expect("start_hand failed");

    // кто сейчас ходит
    let current_seat = engine.current_actor.expect("no current actor");
    let player = table.seats[current_seat as usize]
        .as_ref()
        .unwrap()
        .player_id;

    let action = PlayerAction {
        player_id: player,
        seat: current_seat,
        kind: PlayerActionKind::Fold,
    };

    let status = apply_action(&mut table, &mut engine, action).expect("apply_action failed");

    match status {
        HandStatus::Finished(summary, _history) => {
            assert_eq!(summary.table_id, table.id);
            assert_eq!(summary.hand_id, 1);
            assert_eq!(summary.street_reached, Street::Showdown);
            assert!(!table.hand_in_progress);
            // total_pot должен быть > 0 (blinds)
            assert!(summary.total_pot.0 > 0);
        }
        HandStatus::Ongoing => {
            panic!("heads-up fold должен завершать раздачу");
        }
    }
}
