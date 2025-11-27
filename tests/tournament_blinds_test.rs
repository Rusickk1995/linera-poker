// tests/tournament_blinds_test.rs

use poker_engine::domain::{
    blinds::BlindStructure,
    chips::Chips,
    tournament::{
        TableBalancingConfig, Tournament, TournamentConfig, TournamentScheduleConfig,
        TournamentStatus,
    },
    PlayerId, TableId, TournamentId,
};
use poker_engine::tournament::TournamentRuntime;

#[test]
fn blind_structure_level_for_elapsed() {
    let structure = BlindStructure::simple_demo_structure();

    let lvl1 = structure.level_for_elapsed_minutes(0);
    assert_eq!(lvl1.level, 1);

    let lvl1_bis = structure.level_for_elapsed_minutes(9);
    assert_eq!(lvl1_bis.level, 1);

    let lvl2 = structure.level_for_elapsed_minutes(10);
    assert_eq!(lvl2.level, 2);

    let last = structure.level_for_elapsed_minutes(10_000);
    assert_eq!(last.level, structure.levels.last().unwrap().level);
}

fn demo_schedule() -> TournamentScheduleConfig {
    TournamentScheduleConfig {
        scheduled_start_ts: 0,
        allow_start_earlier: true,
        break_every_minutes: 60,
        break_duration_minutes: 5,
    }
}

fn demo_balancing() -> TableBalancingConfig {
    TableBalancingConfig {
        enabled: true,
        max_seat_diff: 1,
    }
}

fn demo_tournament_config() -> TournamentConfig {
    TournamentConfig {
        name: "Demo MTT".into(),
        description: None,
        starting_stack: Chips::new(10_000),
        max_players: 100,
        min_players_to_start: 2,
        table_size: 9,
        freezeout: true,
        reentry_allowed: false,
        max_entries_per_player: 1,
        late_reg_level: 0,
        blind_structure: BlindStructure::simple_demo_structure(),
        auto_approve: true,
        schedule: demo_schedule(),
        balancing: demo_balancing(),
    }
}

#[test]
fn create_tournament_and_register_players() {
    let cfg = demo_tournament_config();
    let mut t =
        Tournament::new(1 as TournamentId, 100 as PlayerId, cfg).unwrap();

    assert_eq!(t.status, TournamentStatus::Registering);
    assert_eq!(t.current_level, 1);

    t.register_player(1).unwrap();
    t.register_player(2).unwrap();

    assert_eq!(t.registrations.len(), 2);
}

#[test]
fn build_tables_for_tournament_runtime() {
    let cfg = demo_tournament_config();
    let mut t =
        Tournament::new(1 as TournamentId, 100 as PlayerId, cfg).unwrap();

    for pid in 1u64..=10u64 {
        t.register_player(pid as PlayerId).unwrap();
    }

    let tables =
        TournamentRuntime::build_tables_for_tournament(&t, 1 as TableId);

    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].seats.len(), 9);
    assert_eq!(tables[1].seats.len(), 1);
}
