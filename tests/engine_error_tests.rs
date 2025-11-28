// tests/engine_error_tests.rs
//
// D. Error Handling (10 тестов) — ДОМЕННЫЙ УРОВЕНЬ
//
// Мы тестируем:
//  1) Некорректная BlindStructure -> TournamentError::InvalidConfig
//  2) Регистрация в не-Registering статусе -> TournamentError::InvalidStatus
//  3) Двойная регистрация -> TournamentError::AlreadyRegistered
//  4) Переполнение турнира -> TournamentError::TournamentFull
//  5) mark_player_busted в не-Running статусе -> TournamentError::InvalidStatus
//  6) Попытка bust после автозавершения турнира -> TournamentError::InvalidStatus
//  7) Правильный рост finishing_place при bust
//  8) check_and_finish_if_needed: турнир завершается, когда остаётся 1 игрок
//  9) apply_time_tick до старта турнира ничего не меняет
// 10) compute_rebalance_moves при 1 столе и/или 1 игроке не падает и не двигает никого
//
// Плюс отдельный тест RNG: shuffle с пустым вектором не падает.

use poker_engine::domain::{PlayerId, TournamentId};
use poker_engine::domain::chips::Chips;
use poker_engine::domain::blinds::{BlindLevel, BlindStructure, AnteType};
use poker_engine::domain::tournament::{
    Tournament,
    TournamentConfig,
    TournamentStatus,
    TournamentScheduleConfig,
    TableBalancingConfig,
    TournamentError,
    TournamentTimeEvent,
};
use poker_engine::infra::rng::DeterministicRng;
use poker_engine::engine::RandomSource; // для rng.shuffle

// -----------------------------
// ВСПОМОГАТЕЛЬНЫЕ КОНСТРУКТОРЫ
// -----------------------------

fn basic_blind_structure() -> BlindStructure {
    BlindStructure {
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
    }
}

fn invalid_blind_structure() -> BlindStructure {
    // level 1 продублирован — это должно ломать validate_full()
    BlindStructure {
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
                level: 1,
                small_blind: Chips(100),
                big_blind: Chips(200),
                ante: Chips(0),
                ante_type: AnteType::None,
                duration_minutes: 10,
            },
        ],
    }
}

fn base_schedule() -> TournamentScheduleConfig {
    TournamentScheduleConfig {
        scheduled_start_ts: 0,
        allow_start_earlier: true,
        break_every_minutes: 60,
        break_duration_minutes: 5,
    }
}

fn base_balancing() -> TableBalancingConfig {
    TableBalancingConfig {
        enabled: true,
        max_seat_diff: 1,
    }
}

fn base_tournament_config_with_blinds(blinds: BlindStructure) -> TournamentConfig {
    TournamentConfig {
        name: "TestTournament".into(),
        description: None,
        starting_stack: Chips(10_000),
        max_players: 3,
        min_players_to_start: 2,
        table_size: 9,
        freezeout: true,
        reentry_allowed: false,
        max_entries_per_player: 1,
        late_reg_level: 0,
        blind_structure: blinds,
        auto_approve: true,
        schedule: base_schedule(),
        balancing: base_balancing(),
    }
}

fn base_tournament_config() -> TournamentConfig {
    base_tournament_config_with_blinds(basic_blind_structure())
}

fn create_tournament(id: TournamentId, owner: PlayerId) -> Tournament {
    let cfg = base_tournament_config();
    Tournament::new(id, owner, cfg).expect("Tournament::new must succeed in tests")
}

// -----------------------------
// 1) Некорректная BlindStructure
// -----------------------------

#[test]
fn invalid_blind_structure_causes_invalid_config_error() {
    let cfg = base_tournament_config_with_blinds(invalid_blind_structure());

    let res = cfg.validate_full();

    assert!(
        matches!(res, Err(TournamentError::InvalidConfig(_))),
        "Некорректная BlindStructure должна приводить к TournamentError::InvalidConfig"
    );
}

// ---------------------------------------------
// 2) Регистрация в не-Registering статусе
// ---------------------------------------------

#[test]
fn register_player_in_non_registering_status_returns_error() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(100, owner);

    // Насильно помечаем Running
    t.status = TournamentStatus::Running;

    let p: PlayerId = 10;
    let res = t.register_player(p);

    assert!(
        matches!(
            res,
            Err(TournamentError::InvalidStatus {
                expected: TournamentStatus::Registering,
                found: TournamentStatus::Running
            })
        ),
        "Регистрация в статусе, отличном от Registering, должна давать TournamentError::InvalidStatus"
    );
}

// ---------------------------------------------
// 3) Двойная регистрация -> AlreadyRegistered
// ---------------------------------------------

#[test]
fn registering_same_player_twice_returns_already_registered_error() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(101, owner);

    let p: PlayerId = 10;
    t.register_player(p).expect("first registration must succeed");

    let res = t.register_player(p);

    assert!(
        matches!(
            res,
            Err(TournamentError::AlreadyRegistered { player_id, tournament_id })
                if player_id == p && tournament_id == t.id
        ),
        "Вторая регистрация того же игрока должна вернуть TournamentError::AlreadyRegistered"
    );
}

// ---------------------------------------------
// 4) Переполнение турнира -> TournamentFull
// ---------------------------------------------

#[test]
fn registering_more_than_max_players_returns_tournament_full() {
    let owner: PlayerId = 1;
    let mut cfg = base_tournament_config();
    cfg.max_players = 2; // ограничим
    let mut t = Tournament::new(102, owner, cfg).unwrap();

    t.register_player(10).unwrap();
    t.register_player(11).unwrap();

    let res = t.register_player(12);

    assert!(
        matches!(
            res,
            Err(TournamentError::TournamentFull { tournament_id })
                if tournament_id == t.id
        ),
        "Регистрация сверх max_players должна давать TournamentError::TournamentFull"
    );
}

// ------------------------------------------------------
// 5) mark_player_busted в не-Running статусе
// ------------------------------------------------------

#[test]
fn mark_player_busted_in_non_running_status_returns_error() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(103, owner);

    t.register_player(10).unwrap();
    t.register_player(11).unwrap();

    // Статус пока Registering, а не Running.
    let res = t.mark_player_busted(10);

    assert!(
        matches!(
            res,
            Err(TournamentError::InvalidStatus {
                expected: TournamentStatus::Running,
                found: TournamentStatus::Registering
            })
        ),
        "mark_player_busted в статусе Registering должен вернуть TournamentError::InvalidStatus"
    );
}

// ------------------------------------------------------
// 6) Попытка bust после автозавершения турнира
// ------------------------------------------------------
//
// Реальное поведение движка:
// - при предпоследнем bust остаётся 1 активный игрок, tournament.finish()
//   переводит статус в Finished и ставит winner_id;
// - дальнейшие вызовы mark_player_busted всегда получают InvalidStatus
//   (expected Running, found Finished).
//
// Это тоже корректная защита, и мы её фиксируем тестом.

#[test]
fn cannot_bust_when_tournament_already_finished() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(104, owner);

    let p1: PlayerId = 10;
    let p2: PlayerId = 11;

    t.register_player(p1).unwrap();
    t.register_player(p2).unwrap();

    // Переводим в Running вручную и фиксируем total_entries
    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    // Выбиваем одного – здесь турнир ещё Running,
    // после mark_player_busted он сам завершится (останется 1 активный).
    let place_p1 = t.mark_player_busted(p1).expect("first bust must succeed");
    assert!(place_p1 > 0, "Игрок должен получить валидное place");
    assert!(
        t.is_finished(),
        "После bust предпоследнего игрока турнир должен перейти в Finished"
    );

    // Попытка ещё раз кого-то bust-нуть, когда статус уже Finished,
    // должна вернуть InvalidStatus (ожидаем Running, получили Finished).
    let res = t.mark_player_busted(p2);

    assert!(
        matches!(
            res,
            Err(TournamentError::InvalidStatus {
                expected: TournamentStatus::Running,
                found: TournamentStatus::Finished
            })
        ),
        "При статусе Finished вызов mark_player_busted должен возвращать TournamentError::InvalidStatus"
    );
}

// ------------------------------------------------------
// 7) finishing_place растёт при последовательных bust
// ------------------------------------------------------

#[test]
fn finishing_place_increases_as_players_bust() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(105, owner);

    let p1: PlayerId = 10;
    let p2: PlayerId = 11;
    let p3: PlayerId = 12;

    t.register_player(p1).unwrap();
    t.register_player(p2).unwrap();
    t.register_player(p3).unwrap();

    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32; // 3

    // Первый вылетевший должен получить place 3
    let place1 = t.mark_player_busted(p1).unwrap();
    assert_eq!(
        place1, 3,
        "Первый вылетевший при total_entries=3 должен получить 3 место"
    );

    // Второй вылетевший — place 2
    let place2 = t.mark_player_busted(p2).unwrap();
    assert_eq!(
        place2, 2,
        "Второй вылетевший должен получить 2 место"
    );

    // Оставшийся игрок становится победителем
    assert!(t.is_finished(), "После двух bust из трёх турнир должен завершиться");
    assert_eq!(
        t.winner_id,
        Some(p3),
        "Последний живой игрок должен стать winner_id"
    );
}

// ------------------------------------------------------
// 8) check_and_finish_if_needed: когда остаётся 1 игрок
// ------------------------------------------------------

#[test]
fn tournament_finishes_when_single_player_left() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(106, owner);

    let p1: PlayerId = 10;
    let p2: PlayerId = 11;

    t.register_player(p1).unwrap();
    t.register_player(p2).unwrap();

    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    // Выбиваем p1 – остаётся один активный
    let _place1 = t.mark_player_busted(p1).unwrap();

    assert!(
        t.is_finished(),
        "Когда остаётся один активный игрок, турнир должен завершиться"
    );
    assert_eq!(
        t.winner_id,
        Some(p2),
        "Единственный оставшийся игрок должен быть winner_id"
    );
}

// ------------------------------------------------------
// 9) apply_time_tick до старта турнира ничего не меняет
// ------------------------------------------------------

#[test]
fn time_tick_before_tournament_start_produces_no_event() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(107, owner);

    // Статус Registering, started_at_ts = None
    let now_ts: u64 = 1_000_000;

    let ev = t.apply_time_tick(now_ts);

    assert!(
        matches!(ev, TournamentTimeEvent::None),
        "Пока турнир не стартовал, apply_time_tick должен возвращать TournamentTimeEvent::None"
    );
    assert_eq!(
        t.current_level, 1,
        "current_level до старта не должен меняться"
    );
}

// ------------------------------------------------------
// 10) compute_rebalance_moves при 1 столе / 1 игроке
// ------------------------------------------------------

#[test]
fn rebalance_with_single_table_and_single_player_has_no_moves() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(108, owner);

    let p: PlayerId = 10;
    t.register_player(p).unwrap();

    // Переводим в Running и рассаживаем по столам
    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    // table_size = 9, next_table_id = 100
    let seating = t.seat_players_evenly(9, 100);

    // Должен быть один стол и один игрок
    assert_eq!(seating.len(), 1);
    assert_eq!(seating[0].1.len(), 1);

    // compute_rebalance_moves при одной таблице / одном игроке
    // не должен падать и не должен генерить перестановки
    let moves = t.compute_rebalance_moves();
    assert!(
        moves.is_empty(),
        "При одном столе и одном игроке ребаланс не должен генерировать перестановки"
    );

    // apply_rebalance_moves тоже не должен ломать состояние
    t.apply_rebalance_moves(&moves);

    // Дополнительно проверим, что игрок по-прежнему активен и где-то сидит
    let regs: Vec<_> = t.active_players().collect();
    assert_eq!(regs.len(), 1);
    assert_eq!(regs[0].player_id, p);
}

// ------------------------------------------------------
// Дополнительно: RNG edge-case (shuffle с пустым массивом)
// ------------------------------------------------------

#[test]
fn deterministic_rng_shuffle_empty_vec_is_safe() {
    let mut rng = DeterministicRng::from_u64(123);
    let mut data: Vec<u32> = Vec::new();

    rng.shuffle(&mut data);

    assert!(
        data.is_empty(),
        "shuffle(&mut empty_vec) не должен менять размер и не должен падать"
    );
}
