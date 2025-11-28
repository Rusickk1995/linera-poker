// tests/tournament_balancing_tests.rs
//
// Контрольные тесты турнирной логики + балансировки столов.
//
// Проверяем:
//  1) Рассадка при 10 игроках и table_size=9 -> 2 стола по 5 игроков (баланс, max_seat_diff = 1).
//  2) compute_rebalance_moves на сильно разбалансированных столах даёт перемещения,
//     а apply_rebalance_moves выравнивает столы до допустимой разницы.
//  3) mark_player_busted помечает игрока как вылетевшего.
//  4) finishing_place растёт при последовательных bust.
//  5) Турнир завершается, когда остаётся один активный игрок.
//  6) apply_rebalance_moves корректно обновляет table_id у игроков.

use poker_engine::domain::{PlayerId, TableId, TournamentId};
use poker_engine::domain::blinds::{AnteType, BlindLevel, BlindStructure};
use poker_engine::domain::chips::Chips;
use poker_engine::domain::tournament::{
    RebalanceMove,
    TableBalancingConfig,
    Tournament,
    TournamentConfig,
    TournamentScheduleConfig,
    TournamentStatus,
};

/// Базовая структура блайндов для тестов.
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

/// Базовое расписание: старт по кнопке, перерыв раз в час на 5 минут.
fn base_schedule() -> TournamentScheduleConfig {
    TournamentScheduleConfig {
        scheduled_start_ts: 0,
        allow_start_earlier: true,
        break_every_minutes: 60,
        break_duration_minutes: 5,
    }
}

/// Балансировка включена, max_seat_diff = 1.
fn base_balancing() -> TableBalancingConfig {
    TableBalancingConfig {
        enabled: true,
        max_seat_diff: 1,
    }
}

/// Базовый конфиг турнира для тестов.
fn base_tournament_config() -> TournamentConfig {
    TournamentConfig {
        name: "BalancingTest".into(),
        description: None,
        starting_stack: Chips(10_000),
        max_players: 100,
        min_players_to_start: 2,
        table_size: 9,
        freezeout: true,
        reentry_allowed: false,
        max_entries_per_player: 1,
        late_reg_level: 0,
        blind_structure: basic_blind_structure(),
        auto_approve: true,
        schedule: base_schedule(),
        balancing: base_balancing(),
    }
}

/// Удобный хелпер для создания турнира.
fn create_tournament(id: TournamentId, owner: PlayerId) -> Tournament {
    let cfg = base_tournament_config();
    Tournament::new(id, owner, cfg).expect("Tournament::new must succeed in tests")
}

// -----------------------------------------------------------------------------
// 1) Рассадка 10 игроков при table_size=9 → 2 стола по 5 игроков
// -----------------------------------------------------------------------------

#[test]
fn seating_two_tables_balanced_5_and_5() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(200, owner);

    // Регистрируем 10 игроков.
    for pid in 1..=10 {
        t.register_player(pid).expect("registration must succeed");
    }

    // Рассаживаем игроков. table_size=9, balancing.enabled=true, max_seat_diff=1.
    let seating = t.seat_players_evenly(9, 100);

    // Должно получиться 2 стола.
    assert_eq!(seating.len(), 2, "Должно быть ровно два стола");

    let first_count = seating[0].1.len();
    let second_count = seating[1].1.len();

    assert_eq!(
        first_count + second_count,
        10,
        "Всего за столами должно быть 10 игроков"
    );

    // Баланс: 5 и 5.
    assert_eq!(
        first_count, 5,
        "Первый стол должен иметь 5 игроков при сбалансированной рассадке"
    );
    assert_eq!(
        second_count, 5,
        "Второй стол должен иметь 5 игроков при сбалансированной рассадке"
    );

    let diff = if first_count > second_count {
        first_count - second_count
    } else {
        second_count - first_count
    };
    assert!(
        diff <= t.config.balancing.max_seat_diff as usize,
        "Разница по количеству игроков между столами не должна превышать max_seat_diff"
    );

    // Дополнительно проверим, что игроки действительно имеют table_id/seat_index.
    for (table_id, players) in seating.iter() {
        for pid in players {
            let reg = t
                .registrations
                .get(pid)
                .expect("registration must exist for seated player");
            assert_eq!(
                reg.table_id,
                Some(*table_id),
                "table_id в Tournament должен совпадать с seating"
            );
            assert!(
                reg.seat_index.is_some(),
                "seat_index должен быть установлен при рассадке"
            );
        }
    }
}

// -----------------------------------------------------------------------------
// 2) compute_rebalance_moves на разбалансированных столах
// -----------------------------------------------------------------------------

#[test]
fn rebalance_moves_reduce_imbalance_between_tables() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(201, owner);

    // 10 игроков, чтобы можно было сделать сильную разбалансировку.
    for pid in 1..=10 {
        t.register_player(pid).unwrap();
    }

    // Сначала рассаживаем их ровно (2 стола по 5).
    let seating = t.seat_players_evenly(9, 200);
    assert_eq!(seating.len(), 2);

    let t1 = seating[0].0;
    let t2 = seating[1].0;

    // ИСКУССТВЕННО создаём сильную разбалансировку:
    // Перенесём трёх игроков с t2 на t1.
    // В итоге получится, например, 8 vs 2.
    let mut moved = 0usize;
    for pid in seating[1].1.iter() {
        if moved >= 3 {
            break;
        }
        if let Some(reg) = t.registrations.get_mut(pid) {
            reg.table_id = Some(t1);
        }
        moved += 1;
    }

    // Посчитаем текущую загрузку столов.
    let mut c1 = 0usize;
    let mut c2 = 0usize;

    for reg in t.active_players() {
        match reg.table_id {
            Some(id) if id == t1 => c1 += 1,
            Some(id) if id == t2 => c2 += 1,
            _ => {}
        }
    }

    assert!(
        (c1 as isize - c2 as isize).unsigned_abs() > t.config.balancing.max_seat_diff as usize,
        "Мы специально создаём разбалансировку больше max_seat_diff"
    );

    // Считаем ребаланс.
    let moves = t.compute_rebalance_moves();
    assert!(
        !moves.is_empty(),
        "При сильной разбалансировке compute_rebalance_moves должен вернуть хотя бы одно перемещение"
    );

    // Применяем ребаланс.
    t.apply_rebalance_moves(&moves);

    // Пересчитываем загрузку.
    let mut c1_after = 0usize;
    let mut c2_after = 0usize;

    for reg in t.active_players() {
        match reg.table_id {
            Some(id) if id == t1 => c1_after += 1,
            Some(id) if id == t2 => c2_after += 1,
            _ => {}
        }
    }

    let diff_after = if c1_after > c2_after {
        c1_after - c2_after
    } else {
        c2_after - c1_after
    };

    assert!(
        diff_after <= t.config.balancing.max_seat_diff as usize,
        "После apply_rebalance_moves разница по игрокам между столами должна быть <= max_seat_diff"
    );
}

// -----------------------------------------------------------------------------
// 3) mark_player_busted помечает игрока как вылетевшего
// -----------------------------------------------------------------------------

#[test]
fn bust_player_marks_as_busted() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(202, owner);

    let p1: PlayerId = 10;
    let p2: PlayerId = 11;

    t.register_player(p1).unwrap();
    t.register_player(p2).unwrap();

    // Переводим в Running и фиксируем total_entries.
    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    let place = t.mark_player_busted(p1).expect("mark_player_busted must succeed");
    assert!(place > 0, "finishing_place должен быть > 0");

    let reg = t
        .registrations
        .get(&p1)
        .expect("player must exist after bust");
    assert!(reg.is_busted, "is_busted должен быть true после bust");
    assert!(
        reg.finishing_place.is_some(),
        "finishing_place должен быть установлен после bust"
    );
}

// -----------------------------------------------------------------------------
// 4) finishing_place растёт при последовательных bust
// -----------------------------------------------------------------------------

#[test]
fn finishing_places_increment() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(203, owner);

    let p1: PlayerId = 10;
    let p2: PlayerId = 11;
    let p3: PlayerId = 12;

    t.register_player(p1).unwrap();
    t.register_player(p2).unwrap();
    t.register_player(p3).unwrap();

    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32; // 3

    let place1 = t.mark_player_busted(p1).unwrap();
    let place2 = t.mark_player_busted(p2).unwrap();

    assert_eq!(place1, 3, "Первый вылетевший при 3 игроках должен получить 3 место");
    assert_eq!(place2, 2, "Второй вылетевший должен получить 2 место");
}

// -----------------------------------------------------------------------------
// 5) Турнир завершается, когда остаётся один активный игрок
// -----------------------------------------------------------------------------

#[test]
fn tournament_finishes_when_one_left() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(204, owner);

    let p1: PlayerId = 10;
    let p2: PlayerId = 11;

    t.register_player(p1).unwrap();
    t.register_player(p2).unwrap();

    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

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

// -----------------------------------------------------------------------------
// 6) apply_rebalance_moves корректно обновляет table_id у игроков
// -----------------------------------------------------------------------------

#[test]
fn apply_rebalance_moves_updates_table_id() {
    let owner: PlayerId = 1;
    let mut t = create_tournament(205, owner);

    let p1: PlayerId = 10;
    let p2: PlayerId = 11;

    t.register_player(p1).unwrap();
    t.register_player(p2).unwrap();

    // Инициализируем столы вручную.
    let t1: TableId = 300;
    let t2: TableId = 301;

    {
        let reg1 = t.registrations.get_mut(&p1).unwrap();
        reg1.table_id = Some(t1);

        let reg2 = t.registrations.get_mut(&p2).unwrap();
        reg2.table_id = Some(t2);
    }

    // Перенесём p2 со стола t2 на t1.
    let moves = vec![RebalanceMove {
        player_id: p2,
        from_table: t2,
        to_table: t1,
    }];

    t.apply_rebalance_moves(&moves);

    let reg2_after = t.registrations.get(&p2).unwrap();
    assert_eq!(
        reg2_after.table_id,
        Some(t1),
        "После apply_rebalance_moves игрок должен оказаться за столом to_table"
    );
}
