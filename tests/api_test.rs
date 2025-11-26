use poker_engine::{
    api::{
        commands::{
            AdjustStackCommand, AnteTypeApi, Command, CreateTableCommand, PlayerActionCommand,
            SeatPlayerCommand, StartHandCommand, TableCommand, TournamentCommand,
            UnseatPlayerCommand,
        },
        dto::{
            CommandResponse, HandHistoryItemDto, HandPlayerResultDto, TableViewDto,
            TournamentViewDto,
        },
        errors::ApiError,
        queries::{build_table_view, Query, QueryResponse},
    },
    domain::{
        card::{Card, Rank, Suit},
        chips::Chips,
        hand::{HandRank, HandSummary, PlayerHandResult, Street},
        player::{PlayerAtTable, PlayerStatus},
        table::{Table, TableConfig, TableStakes, TableType},
        PlayerId, TableId, TournamentId,
    },
    engine::{
        betting::BettingState,
        game_loop::HandEngine,
        pot::Pot,
        HandHistory, HandStatus,
    },
    engine::errors::EngineError,
};

use poker_engine::domain::blinds::AnteType;

/// Утилита: создать минимальный конфиг стола (6-max).
fn make_table_config() -> TableConfig {
    TableConfig {
        max_seats: 6,
        table_type: TableType::Cash,
        stakes: TableStakes::new(
            Chips::new(50),  // SB
            Chips::new(100), // BB
            AnteType::None,
            Chips::ZERO,
        ),
        allow_straddle: false,
        allow_run_it_twice: false,
    }
}


/// Утилита: создать пустой стол.
fn make_empty_table(id: TableId) -> Table {
    Table::new(id, format!("Table {id}"), make_table_config())
}

/// Утилита: карта удобным конструктором.
fn card(rank: Rank, suit: Suit) -> Card {
    Card { rank, suit }
}

// ----------------------
// tests для commands.rs
// ----------------------

#[test]
fn create_table_command_can_be_wrapped_in_top_level_command() {
    let cmd = CreateTableCommand {
        table_id: 42,
        name: "Test Table".to_string(),
        max_seats: 9,
        small_blind: Chips::new(50),
        big_blind: Chips::new(100),
        ante: Chips::new(0),
        ante_type: AnteTypeApi::None,
    };

    let top = Command::CreateTable(cmd);
    match top {
        Command::CreateTable(inner) => {
            assert_eq!(inner.table_id, 42);
            assert_eq!(inner.name, "Test Table");
            assert_eq!(inner.max_seats, 9);
            assert_eq!(inner.small_blind.0, 50);
            assert_eq!(inner.big_blind.0, 100);
        }
        _ => panic!("Expected Command::CreateTable"),
    }
}

#[test]
fn table_commands_variants_can_be_constructed() {
    let seat_cmd = SeatPlayerCommand {
        table_id: 1,
        player_id: 10,
        seat_index: 2,
        display_name: "Ruslan".to_string(),
        initial_stack: Chips::new(10_000),
    };

    let unseat_cmd = UnseatPlayerCommand {
        table_id: 1,
        seat_index: 2,
    };

    let adjust_cmd = AdjustStackCommand {
        table_id: 1,
        seat_index: 2,
        delta: -500,
    };

    let start_cmd = StartHandCommand {
        table_id: 1,
        hand_id: 777,
    };

    // action-команда через движковый PlayerAction
    use poker_engine::engine::actions::{PlayerAction, PlayerActionKind};
    let action_cmd = PlayerActionCommand {
        table_id: 1,
        action: PlayerAction {
            player_id: 10,
            seat: 2,
            kind: PlayerActionKind::Call,
        },
    };

    let c1 = TableCommand::SeatPlayer(seat_cmd);
    let c2 = TableCommand::UnseatPlayer(unseat_cmd);
    let c3 = TableCommand::AdjustStack(adjust_cmd);
    let c4 = TableCommand::StartHand(start_cmd);
    let c5 = TableCommand::PlayerAction(action_cmd);

    // просто проверяем, что всё живёт в enum без паники
    match c1 {
        TableCommand::SeatPlayer(sc) => {
            assert_eq!(sc.player_id, 10);
            assert_eq!(sc.seat_index, 2);
        }
        _ => panic!("Expected SeatPlayer"),
    }

    match c2 {
        TableCommand::UnseatPlayer(uc) => {
            assert_eq!(uc.seat_index, 2);
        }
        _ => panic!("Expected UnseatPlayer"),
    }

    match c3 {
        TableCommand::AdjustStack(ac) => {
            assert_eq!(ac.delta, -500);
        }
        _ => panic!("Expected AdjustStack"),
    }

    match c4 {
        TableCommand::StartHand(sh) => {
            assert_eq!(sh.hand_id, 777);
        }
        _ => panic!("Expected StartHand"),
    }

    match c5 {
        TableCommand::PlayerAction(pa) => {
            assert_eq!(pa.table_id, 1);
            assert_eq!(pa.action.player_id, 10);
        }
        _ => panic!("Expected PlayerAction"),
    }
}

#[test]
fn tournament_command_placeholder_exists() {
    let t_cmd = TournamentCommand::Placeholder;
    match t_cmd {
        TournamentCommand::Placeholder => {}
    }
}

// ----------------------
// tests для dto.rs
// ----------------------

#[test]
fn table_view_dto_basic_fields() {
    let dto = TableViewDto {
        table_id: 5,
        name: "Example".to_string(),
        max_seats: 9,
        small_blind: Chips::new(50),
        big_blind: Chips::new(100),
        ante: Chips::new(0),
        street: Street::Preflop,
        dealer_button: Some(3),
        total_pot: Chips::new(1000),
        board: vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Hearts),
        ],
        players: Vec::new(),
        hand_in_progress: true,
        current_actor_seat: Some(4),
    };

    assert_eq!(dto.table_id, 5);
    assert_eq!(dto.max_seats, 9);
    assert!(dto.hand_in_progress);
    assert_eq!(dto.current_actor_seat, Some(4));
}

#[test]
fn map_hand_status_to_response_ongoing_returns_table_state() {
    let table_dto = TableViewDto {
        table_id: 1,
        name: "Ongoing".to_string(),
        max_seats: 6,
        small_blind: Chips::new(50),
        big_blind: Chips::new(100),
        ante: Chips::ZERO,
        street: Street::Flop,
        dealer_button: Some(1),
        total_pot: Chips::new(500),
        board: vec![],
        players: Vec::new(),
        hand_in_progress: true,
        current_actor_seat: Some(2),
    };

    let resp = poker_engine::api::dto::map_hand_status_to_response(
        HandStatus::Ongoing,
        table_dto.clone(),
    );

    match resp {
        CommandResponse::TableState(t) => {
            assert_eq!(t.table_id, 1);
            assert!(t.hand_in_progress);
        }
        _ => panic!("Expected TableState for HandStatus::Ongoing"),
    }
}

#[test]
fn map_hand_status_to_response_finished_builds_history_item() {
    let summary = HandSummary {
        hand_id: 100,
        table_id: 1,
        street_reached: Street::River,
        board: vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Spades),
            card(Rank::Queen, Suit::Spades),
            card(Rank::Jack, Suit::Spades),
            card(Rank::Ten, Suit::Spades),
        ],
        total_pot: Chips::new(10_000),
        results: vec![
            PlayerHandResult {
                player_id: 1,
                rank: Some(HandRank(123)),
                net_chips: Chips::new(10_000),
                is_winner: true,
            },
            PlayerHandResult {
                player_id: 2,
                rank: Some(HandRank(50)),
                net_chips: Chips::ZERO,
                is_winner: false,
            },
        ],
    };

    let history = HandHistory { events: vec![] };

    let table_dto = TableViewDto {
        table_id: 1,
        name: "Finished".to_string(),
        max_seats: 6,
        small_blind: Chips::new(50),
        big_blind: Chips::new(100),
        ante: Chips::ZERO,
        street: Street::River,
        dealer_button: Some(3),
        total_pot: Chips::new(10_000),
        board: summary.board.clone(),
        players: Vec::new(),
        hand_in_progress: false,
        current_actor_seat: None,
    };

    let resp = poker_engine::api::dto::map_hand_status_to_response(
        HandStatus::Finished(summary, history),
        table_dto,
    );

    match resp {
        CommandResponse::HandFinished { table, history } => {
            assert_eq!(table.table_id, 1);
            let hist: HandHistoryItemDto = history.expect("history should be Some");
            assert_eq!(hist.hand_id, 100);
            assert_eq!(hist.total_pot.0, 10_000);
            assert_eq!(hist.players.len(), 2);
            assert!(hist.players.iter().any(|p| p.is_winner));
        }
        _ => panic!("Expected HandFinished for HandStatus::Finished"),
    }
}

#[test]
fn tournament_view_dto_holds_basic_info() {
    let dto = TournamentViewDto {
        tournament_id: 7,
        name: "Sunday Major".to_string(),
        status: "Running".to_string(),
        current_level: 10,
        players_registered: 123,
        tables_running: 12,
    };

    assert_eq!(dto.tournament_id, 7);
    assert_eq!(dto.current_level, 10);
    assert_eq!(dto.players_registered, 123);
}

// ----------------------
// tests для errors.rs
// ----------------------

#[test]
fn api_error_from_engine_error_wraps_message() {
    let engine_err = EngineError::NotEnoughPlayers;
    let api_err: ApiError = engine_err.into();

    match api_err {
        ApiError::EngineError(msg) => {
            assert!(
                msg.contains("Недостаточно активных игроков"),
                "unexpected message: {msg}"
            );
        }
        _ => panic!("Expected ApiError::EngineError"),
    }
}

#[test]
fn api_error_variants_exist_and_hold_data() {
    let e1 = ApiError::BadRequest("oops".to_string());
    let e2 = ApiError::TableNotFound(10);
    let e3 = ApiError::PlayerNotAtTable(5);
    let e4 = ApiError::InvalidCommand("bad state".to_string());
    let e5 = ApiError::Internal("boom".to_string());

    match e1 {
        ApiError::BadRequest(s) => assert_eq!(s, "oops"),
        _ => panic!("Expected BadRequest"),
    }
    match e2 {
        ApiError::TableNotFound(id) => assert_eq!(id, 10),
        _ => panic!("Expected TableNotFound"),
    }
    match e3 {
        ApiError::PlayerNotAtTable(pid) => assert_eq!(pid, 5),
        _ => panic!("Expected PlayerNotAtTable"),
    }
    match e4 {
        ApiError::InvalidCommand(s) => assert_eq!(s, "bad state"),
        _ => panic!("Expected InvalidCommand"),
    }
    match e5 {
        ApiError::Internal(s) => assert_eq!(s, "boom"),
        _ => panic!("Expected Internal"),
    }
}

// ----------------------
// tests для queries.rs
// ----------------------

#[test]
fn query_variants_construct_correctly() {
    let q1 = Query::GetTable { table_id: 1 };
    let q2 = Query::ListTables;
    let q3 = Query::GetTournament { tournament_id: 7 };

    match q1 {
        Query::GetTable { table_id } => assert_eq!(table_id, 1),
        _ => panic!("Expected GetTable"),
    }
    match q2 {
        Query::ListTables => {}
        _ => panic!("Expected ListTables"),
    }
    match q3 {
        Query::GetTournament { tournament_id } => assert_eq!(tournament_id, 7),
        _ => panic!("Expected GetTournament"),
    }
}

#[test]
fn query_response_variants_hold_data() {
    let table_view = TableViewDto {
        table_id: 1,
        name: "QResp".to_string(),
        max_seats: 6,
        small_blind: Chips::new(50),
        big_blind: Chips::new(100),
        ante: Chips::ZERO,
        street: Street::Preflop,
        dealer_button: None,
        total_pot: Chips::ZERO,
        board: Vec::new(),
        players: Vec::new(),
        hand_in_progress: false,
        current_actor_seat: None,
    };

    let tview = TournamentViewDto {
        tournament_id: 3,
        name: "T".to_string(),
        status: "Registering".to_string(),
        current_level: 1,
        players_registered: 0,
        tables_running: 0,
    };

    let r1 = QueryResponse::Table(table_view.clone());
    let r2 = QueryResponse::Tables(vec![table_view.clone()]);
    let r3 = QueryResponse::TournamentInfo(tview);

    match r1 {
        QueryResponse::Table(t) => assert_eq!(t.table_id, 1),
        _ => panic!("Expected Table"),
    }
    match r2 {
        QueryResponse::Tables(list) => {
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].name, "QResp");
        }
        _ => panic!("Expected Tables"),
    }
    match r3 {
        QueryResponse::TournamentInfo(info) => {
            assert_eq!(info.tournament_id, 3);
            assert_eq!(info.status, "Registering");
        }
        _ => panic!("Expected TournamentInfo"),
    }
}

#[test]
fn build_table_view_maps_basic_fields_without_engine() {
    let mut table = make_empty_table(1);
    table.total_pot = Chips::new(500);
    table.hand_in_progress = true;
    table.dealer_button = Some(0);

    // добавляем одного игрока
    table.seats[0] = Some(PlayerAtTable {
        player_id: 10,
        stack: Chips::new(2000),
        current_bet: Chips::new(100),
        status: PlayerStatus::Active,
        hole_cards: vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::Ace, Suit::Hearts),
        ],
    });

    let dto = build_table_view(
        &table,
        None,
        |pid: PlayerId| format!("Player {pid}"),
        |_pid: PlayerId| true, // герой → показываем карты
    );

    assert_eq!(dto.table_id, 1);
    assert_eq!(dto.total_pot.0, 500);
    assert_eq!(dto.players.len(), 1);
    let p = &dto.players[0];
    assert_eq!(p.player_id, 10);
    assert_eq!(p.display_name, "Player 10");
    assert!(p.hole_cards.is_some());
}

#[test]
fn build_table_view_uses_engine_current_actor_and_hides_non_hero_cards() {
    let mut table = make_empty_table(2);
    table.hand_in_progress = true;
    table.dealer_button = Some(0);

    // два игрока
    table.seats[0] = Some(PlayerAtTable {
        player_id: 1,
        stack: Chips::new(1000),
        current_bet: Chips::new(100),
        status: PlayerStatus::Active,
        hole_cards: vec![
            card(Rank::King, Suit::Spades),
            card(Rank::King, Suit::Hearts),
        ],
    });
    table.seats[1] = Some(PlayerAtTable {
        player_id: 2,
        stack: Chips::new(2000),
        current_bet: Chips::new(100),
        status: PlayerStatus::Active,
        hole_cards: vec![
            card(Rank::Queen, Suit::Spades),
            card(Rank::Queen, Suit::Hearts),
        ],
    });

    // делаем минимальный HandEngine только для current_actor
    let engine = HandEngine {
        table_id: table.id,
        hand_id: 999,
        deck: poker_engine::domain::deck::Deck { cards: Vec::new() },
        betting: BettingState::new(
            Street::Preflop,
            Chips::new(100),
            Chips::new(100),
            vec![1], // ход за seat 1
        ),
        pot: Pot::new(),
        side_pots: Vec::new(),
        contributions: std::collections::HashMap::new(),
        current_actor: Some(1),
        history: HandHistory { events: Vec::new() },
    };

    // герой = только player_id 1
    let dto = build_table_view(
        &table,
        Some(&engine),
        |pid: PlayerId| format!("P{pid}"),
        |pid: PlayerId| pid == 1,
    );

    assert_eq!(dto.current_actor_seat, Some(1));
    assert_eq!(dto.players.len(), 2);

    let p1 = dto.players.iter().find(|p| p.player_id == 1).unwrap();
    let p2 = dto.players.iter().find(|p| p.player_id == 2).unwrap();

    assert!(p1.hole_cards.is_some(), "hero cards must be visible");
    assert!(
        p2.hole_cards.is_none(),
        "non-hero cards must be hidden in DTO"
    );
}
