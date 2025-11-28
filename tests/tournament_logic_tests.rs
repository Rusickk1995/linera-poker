// tests/tournament_logic_tests.rs

use poker_engine::domain::{
    Tournament, TournamentConfig, TournamentStatus,
    PlayerId
};
use poker_engine::domain::chips::Chips;
use poker_engine::domain::blinds::{BlindLevel, BlindStructure, AnteType};
use poker_engine::domain::tournament::{
    TournamentError, TournamentScheduleConfig, TableBalancingConfig
};

fn sample_config() -> TournamentConfig {
    TournamentConfig {
        name: "Test".into(),
        description: None,
        starting_stack: Chips(10000),
        max_players: 3,
        min_players_to_start: 2,
        table_size: 9,
        freezeout: true,
        reentry_allowed: false,
        max_entries_per_player: 1,
        late_reg_level: 0,

        blind_structure: BlindStructure {
            levels: vec![
                BlindLevel {
                    level: 1,
                    small_blind: Chips(50),
                    big_blind: Chips(100),
                    ante: Chips(0),
                    ante_type: AnteType::None,     // ← ДОБАВЛЕНО
                    duration_minutes: 10,
                }
            ],
        },

        auto_approve: true,

        schedule: TournamentScheduleConfig {
            scheduled_start_ts: 0,
            allow_start_earlier: true,
            break_every_minutes: 60,
            break_duration_minutes: 5,
        },

        balancing: TableBalancingConfig {
            enabled: false,
            max_seat_diff: 1,
        },
    }
}

//
// TEST 1 — регистрация добавляет игроков
//
#[test]
fn registration_adds_players() {
    let owner: PlayerId = 999;
    let cfg = sample_config();

    let mut t = Tournament::new(1, owner, cfg).unwrap();

    t.register_player(10).unwrap();
    t.register_player(20).unwrap();
    t.register_player(30).unwrap();

    let ids: Vec<PlayerId> =
        t.registrations.values().map(|r| r.player_id).collect();

    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&10));
    assert!(ids.contains(&20));
    assert!(ids.contains(&30));
}

//
// TEST 2 — players sorted
//
#[test]
fn registration_players_sorted() {
    let owner = 555;
    let cfg = sample_config();
    let mut t = Tournament::new(1, owner, cfg).unwrap();

    t.register_player(30).unwrap();
    t.register_player(10).unwrap();
    t.register_player(20).unwrap();

    let mut ids: Vec<PlayerId> =
        t.registrations.values().map(|r| r.player_id).collect();

    ids.sort_unstable();

    assert_eq!(ids, vec![10, 20, 30]);
}

//
// TEST 3 — max_players ограничивает
//
#[test]
fn registration_respects_max_players() {
    let owner = 1000;
    let cfg = sample_config(); // max_players = 3

    let mut t = Tournament::new(1, owner, cfg).unwrap();

    t.register_player(1).unwrap();
    t.register_player(2).unwrap();
    t.register_player(3).unwrap();

    let err = t.register_player(4).unwrap_err();

    match err {
        TournamentError::TournamentFull { .. } => {}
        e => panic!("expected TournamentFull, got {:?}", e),
    }
}

//
// TEST 4 — турнир НЕ стартует если мало игроков
//
#[test]
fn tournament_not_ready_with_too_few_players() {
    let owner = 777;
    let cfg = sample_config();

    let mut t = Tournament::new(1, owner, cfg).unwrap();
    let now: u64 = 1000;

    t.register_player(1).unwrap(); // только один игрок

    assert!(!t.can_start_now(now));
}

//
// TEST 5 — турнир стартует при >= min_players
//
#[test]
fn tournament_starts_when_enough_players() {
    let owner = 888;
    let cfg = sample_config();

    let mut t = Tournament::new(1, owner, cfg).unwrap();
    let now: u64 = 5000;

    t.register_player(1).unwrap();
    t.register_player(2).unwrap();

    assert!(t.can_start_now(now));

    t.start(now).expect("must start");

    assert_eq!(t.status, TournamentStatus::Running);
    assert_eq!(t.started_at_ts, Some(now));
    assert_eq!(t.total_entries, 2);
}
