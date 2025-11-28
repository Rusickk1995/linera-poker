// tests/tournament_time_tests.rs
//
// Проверяем:
//
// 1) Blind structure / уровни блайндов:
//    - через X минут уровень повышается;
//    - до X минут уровень не повышается;
//    - ante / ante_type доходят в current_blind_level.
//
// 2) Schedule / breaks:
//    - break logic работает: Running -> OnBreak -> Running;
//    - за 1 минуту до break статус всё ещё Running.

use poker_engine::domain::{
    Tournament, TournamentConfig, TournamentStatus,
};
use poker_engine::domain::tournament::{
    TournamentScheduleConfig, TableBalancingConfig, TournamentTimeEvent,
};
use poker_engine::domain::chips::Chips;
use poker_engine::domain::blinds::{BlindLevel, BlindStructure, AnteType};

//
// Вспомогательный конфиг для тестов уровней блайндов:
// - 2 уровня по 10 минут;
// - без фокуса на перерывы.
//
fn blinds_config_two_levels() -> TournamentConfig {
    TournamentConfig {
        name: "BlindsTwoLevels".into(),
        description: None,
        starting_stack: Chips(10000),
        max_players: 100,
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
                    ante_type: AnteType::None,
                    duration_minutes: 10,
                },
                BlindLevel {
                    level: 2,
                    small_blind: Chips(100),
                    big_blind: Chips(200),
                    ante: Chips(0),
                    ante_type: AnteType::None,
                    duration_minutes: 10,
                },
            ],
        },

        auto_approve: true,

        // Расписание не критично, но должно быть валидным.
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
// Конфиг для теста break-логики:
// - один уровень на "очень долго" (чтобы не ловить LevelAdvanced);
// - короткий цикл break (например, каждые 10 минут + 5 минут перерыв).
//
fn breaks_config_single_level() -> TournamentConfig {
    TournamentConfig {
        name: "BreaksConfig".into(),
        description: None,
        starting_stack: Chips(10000),
        max_players: 100,
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
                    ante: Chips(25),
                    ante_type: AnteType::None,
                    // Делаем уровень "очень длинным", чтобы за время
                    // наших break-тестов уровень не поменялся.
                    duration_minutes: 1000,
                },
            ],
        },

        auto_approve: true,

        schedule: TournamentScheduleConfig {
            scheduled_start_ts: 0,
            allow_start_earlier: true,
            break_every_minutes: 10,
            break_duration_minutes: 5,
        },

        balancing: TableBalancingConfig {
            enabled: true,
            max_seat_diff: 1,
        },
    }
}

//
// TEST 1: до истечения длительности уровня — уровень не меняется.
//
#[test]
fn blind_level_does_not_advance_before_duration() {
    let owner: u64 = 42;
    let cfg = blinds_config_two_levels();
    let mut t = Tournament::new(1, owner, cfg).unwrap();

    // Регистрируем минимум игроков для старта.
    t.register_player(1).unwrap();
    t.register_player(2).unwrap();

    let start_ts: u64 = 1_000_000;
    t.start(start_ts).unwrap();

    assert_eq!(t.current_level, 1);

    // Прошло 9 минут -> всё ещё первый уровень.
    let now_ts = start_ts + 9 * 60;
    let ev = t.apply_time_tick(now_ts);

    assert!(matches!(ev, TournamentTimeEvent::None));
    assert_eq!(t.current_level, 1);
}

//
// TEST 2: после истечения длительности уровня — уровень повышается.
//
#[test]
fn blind_level_advances_after_duration() {
    let owner: u64 = 43;
    let cfg = blinds_config_two_levels();
    let mut t = Tournament::new(1, owner, cfg).unwrap();

    t.register_player(1).unwrap();
    t.register_player(2).unwrap();

    let start_ts: u64 = 2_000_000;
    t.start(start_ts).unwrap();

    assert_eq!(t.current_level, 1);

    // Прошло 11 минут -> уровень должен стать 2.
    let now_ts = start_ts + 11 * 60;
    let ev = t.apply_time_tick(now_ts);

    match ev {
        TournamentTimeEvent::LevelAdvanced { from, to, new_blinds } => {
            assert_eq!(from, 1);
            assert_eq!(to, 2);
            assert_eq!(new_blinds.level, 2);
            assert_eq!(t.current_level, 2);
        }
        other => panic!("ожидали LevelAdvanced, получили {:?}", other),
    }
}

//
// TEST 3: ante / ante_type корректно доступны через current_blind_level.
//
#[test]
fn ante_type_and_value_are_visible_in_current_blind_level() {
    let owner: u64 = 44;

    // Используем breaks_config_single_level, где мы задали ante и ante_type.
    let cfg = breaks_config_single_level();
    let mut t = Tournament::new(1, owner, cfg).unwrap();

    t.register_player(1).unwrap();
    t.register_player(2).unwrap();

    let start_ts: u64 = 3_000_000;
    t.start(start_ts).unwrap();

    let level = t.current_blind_level();

    // Проверяем, что bblind/ante/ante_type совпадают с конфигом.
    assert_eq!(level.level, 1);
    assert_eq!(level.small_blind, Chips(50));
    assert_eq!(level.big_blind, Chips(100));
    assert_eq!(level.ante, Chips(25));
    assert!(matches!(level.ante_type, AnteType::None));
}

//
// TEST 4: break-логика — Running -> OnBreak -> Running,
//         и за 1 минуту до break статус всё ещё Running.
//
#[test]
fn break_logic_enters_and_exits_correctly() {
    let owner: u64 = 55;
    let cfg = breaks_config_single_level();
    let mut t = Tournament::new(1, owner, cfg).unwrap();

    t.register_player(1).unwrap();
    t.register_player(2).unwrap();

    let start_ts: u64 = 4_000_000;
    t.start(start_ts).unwrap();

    assert_eq!(t.status, TournamentStatus::Running);

    // За 1 минуту до break:
    //
    // break_every_minutes = 10
    // -> за 9 минут до этого moment:
    //
    // elapsed = 9 -> cycle_pos = 9 (< 10)
    // статус должен быть Running, эвент = None.
    let before_break_ts = start_ts + 9 * 60;
    let ev_before = t.apply_time_tick(before_break_ts);

    assert!(matches!(ev_before, TournamentTimeEvent::None));
    assert_eq!(t.status, TournamentStatus::Running, "за 1 минуту до break статус должен быть Running");

    // Момент начала break:
    //
    // elapsed = 10 -> cycle_pos = 10 (== break_every),
    // статус должен перейти в OnBreak, событие BreakStarted.
    let break_start_ts = start_ts + 10 * 60;
    let ev_break = t.apply_time_tick(break_start_ts);

    match ev_break {
        TournamentTimeEvent::BreakStarted => {
            assert_eq!(t.status, TournamentStatus::OnBreak);
            assert_eq!(t.current_level, 1, "уровень не должен меняться при входе в break");
        }
        other => panic!("ожидали BreakStarted, получили {:?}", other),
    }

    // Где-то внутри break (например, +12 минут от старта):
    //
    // elapsed = 12 -> cycle_pos = 12 (≥ 10 и < 15),
    // всё ещё OnBreak, событие None.
    let mid_break_ts = start_ts + 12 * 60;
    let ev_mid = t.apply_time_tick(mid_break_ts);

    assert!(matches!(ev_mid, TournamentTimeEvent::None));
    assert_eq!(t.status, TournamentStatus::OnBreak);

    // Выход из break:
    //
    // cycle_minutes = 10 + 5 = 15
    // elapsed = 16 -> cycle_pos = 1 (< 10),
    // статус должен стать Running, событие BreakEnded
    // (так как уровень у нас с duration_minutes = 1000 и не меняется).
    let after_break_ts = start_ts + 16 * 60;
    let ev_after = t.apply_time_tick(after_break_ts);

    match ev_after {
        TournamentTimeEvent::BreakEnded => {
            assert_eq!(t.status, TournamentStatus::Running);
            assert_eq!(t.current_level, 1, "уровень всё ещё 1, потому что duration_minutes очень большой");
        }
        other => panic!("ожидали BreakEnded, получили {:?}", other),
    }
}
