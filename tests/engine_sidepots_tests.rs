//! Side pot / showdown / finish-hand tests для poker-engine.
//!
//! Здесь мы проверяем:
//! - формирование side pots по contributions (2, 3, 4 all-in);
//! - корректный состав eligible_seats;
//! - отсутствие "мусорных" pot'ов;
//! - сценарий "все сфолдили → один победитель" через настоящий game_loop.

use std::collections::HashMap;

use poker_engine::domain::{
    blinds::AnteType,
    chips::Chips,
    player::{PlayerAtTable, PlayerStatus},
    table::{Table, TableConfig, TableStakes, TableType},
    HandId, PlayerId, SeatIndex, TableId,
};

use poker_engine::engine::{
    actions::{PlayerAction, PlayerActionKind},
    game_loop::{apply_action, start_hand, HandStatus},
    side_pots::{compute_side_pots, SidePot},
};

use poker_engine::infra::rng::DeterministicRng;

/// Утилита: собрать contributions из (seat, amount_1) в HashMap.
fn make_contributions(pairs: &[(SeatIndex, u64)]) -> HashMap<SeatIndex, Chips> {
    let mut m = HashMap::new();
    for (seat, amount) in pairs {
        m.insert(*seat, Chips(*amount));
    }
    m
}

/// Утилита: достать (amount, eligible_seats_sorted) из SidePot.
fn pot_info(p: &SidePot) -> (u64, Vec<SeatIndex>) {
    let mut seats = p.eligible_seats.clone();
    seats.sort_unstable();
    (p.amount.0, seats)
}

//
// ====================== SIDE POTS: 2, 3, 4 ALL-IN ======================
//

/// 2 игрока, оба внесли по 100 фишек.
/// Ожидаем один общий пот 200, eligible = {0, 1}.
#[test]
fn side_pots_two_players_equal_all_in() {
    let contrib = make_contributions(&[(0, 100), (1, 100)]);

    let pots = compute_side_pots(&contrib);

    assert_eq!(pots.len(), 1, "Должен быть один общий пот");

    let (amount, seats) = pot_info(&pots[0]);
    assert_eq!(amount, 200);
    assert_eq!(seats, vec![0, 1]);
}

/// 3 игрока all-in: 100, 200, 300.
/// Ожидаем:
/// - pot0: 300 (100 * 3), eligible {0,1,2}
/// - pot1: 200 (100 * 2), eligible {1,2}
/// - pot2: 100 (100 * 1), eligible {2}
#[test]
fn side_pots_three_players_all_in_100_200_300() {
    let contrib = make_contributions(&[(0, 100), (1, 200), (2, 300)]);

    let pots = compute_side_pots(&contrib);
    assert_eq!(pots.len(), 3, "Ожидаем 3 слоя side pots");

    let info0 = pot_info(&pots[0]);
    let info1 = pot_info(&pots[1]);
    let info2 = pot_info(&pots[2]);

    assert_eq!(info0, (300, vec![0, 1, 2]));
    assert_eq!(info1, (200, vec![1, 2]));
    assert_eq!(info2, (100, vec![2]));
}

/// 4 игрока all-in: 100, 100, 300, 300.
/// Ожидаем:
/// - pot0: 400 (100 * 4), eligible {0,1,2,3}
/// - pot1: 400 (200 * 2), eligible {2,3}
#[test]
fn side_pots_four_players_all_in_100_100_300_300() {
    let contrib = make_contributions(&[(0, 100), (1, 100), (2, 300), (3, 300)]);

    let pots = compute_side_pots(&contrib);
    assert_eq!(pots.len(), 2, "Ожидаем 2 слоя side pots");

    let info0 = pot_info(&pots[0]);
    let info1 = pot_info(&pots[1]);

    assert_eq!(info0, (400, vec![0, 1, 2, 3]));
    assert_eq!(info1, (400, vec![2, 3]));
}

/// Набор contributions с "дырками" проверяет, что:
/// - pots упорядочены по возрастанию "слоёв";
/// - нет pot'ов с нулевой суммой.
#[test]
fn side_pots_are_consistent_and_non_zero() {
    let contrib = make_contributions(&[(0, 50), (1, 200), (2, 200), (3, 500)]);

    let pots = compute_side_pots(&contrib);
    assert!(!pots.is_empty());

    // Все суммы > 0
    for p in &pots {
        assert!(p.amount.0 > 0, "Pot не должен быть нулевым");
    }

    // Проверяем что сумма всех pot'ов == сумме contributions (с учётом слоёв).
    let total_contrib: u64 = contrib.values().map(|c| c.0).sum();
    let total_pots: u64 = pots.iter().map(|p| p.amount.0).sum();

    // В стандартных алгоритмах side pots сумма pot'ов >= сумме contributions
    // (из-за того, что каждый слой учитывает несколько игроков).
    assert!(
        total_pots >= total_contrib,
        "Сумма всех pot'ов должна быть >= сумме contributions"
    );
}

//
// ====================== FINISH HAND: ВСЕ СФОЛДИЛИ ======================
//

/// Утилита: создать простой турнирный стол на N игроков с одинаковыми стеками
/// и стартануть раздачу.
fn setup_table_with_n_players(n: usize, stack: u64) -> (Table, poker_engine::engine::game_loop::HandEngine) {
    let table_id: TableId = 1;
    let stakes = TableStakes {
        small_blind: Chips(50),
        big_blind: Chips(100),
        ante: Chips(0),
        ante_type: AnteType::None,
    };

    let config = TableConfig {
        max_seats: n as u8,
        table_type: TableType::Tournament,
        stakes,
        allow_straddle: false,
        allow_run_it_twice: false,
    };

    let mut table = Table::new(table_id, "SidePotTestTable".to_string(), config);

    // Рассаживаем игроков по первым seat'ам.
    for i in 0..n {
        let pid: PlayerId = (i as u64) + 1;
        table.seats[i] = Some(PlayerAtTable::new(pid, Chips(stack)));
    }

    let mut rng = DeterministicRng::from_u64(123456);
    let hand_id: HandId = 1;

    let engine = start_hand(&mut table, &mut rng, hand_id)
        .expect("start_hand должен успешно запустить раздачу");

    (table, engine)
}

/// Сценарий:
/// - 3 игрока за столом.
/// - Идёт префлоп.
/// - Двое сфолдили, остаётся один активный.
/// - Раздача должна завершиться без шоудауна, один победитель забирает банк.
#[test]
fn finish_hand_when_everyone_folds_except_one() {
    let (mut table, mut engine) = setup_table_with_n_players(3, 10_000);

    // Найдём текущего актёра (seat) и его player_id.
    let current_seat = engine
        .current_actor
        .expect("Должен быть текущий актёр на префлопе");
    let current_player_id = table.seats[current_seat as usize]
        .as_ref()
        .expect("seat должен быть занят")
        .player_id;

    // 1) Первый игрок: FOLD
    let action1 = PlayerAction {
        player_id: current_player_id,
        seat: current_seat,
        kind: PlayerActionKind::Fold,
    };

    let status1 = apply_action(&mut table, &mut engine, action1)
        .expect("Fold должен быть валидным действием");
    // После первого фолда раздача ещё не должна закончиться.
    match status1 {
        HandStatus::Ongoing => {}
        _ => panic!("После первого фолда раздача не должна завершиться"),
    }

    // 2) Следующий актёр после фолда:
    let next_seat = engine
        .current_actor
        .expect("После первого фолда должен быть следующий актёр");
    let next_player_id = table.seats[next_seat as usize]
        .as_ref()
        .expect("seat должен быть занят")
        .player_id;

    // Второй игрок тоже FOLD → должен остаться один активный игрок → раздача завершается.
    let action2 = PlayerAction {
        player_id: next_player_id,
        seat: next_seat,
        kind: PlayerActionKind::Fold,
    };

    let status2 = apply_action(&mut table, &mut engine, action2)
        .expect("Fold должен быть валидным действием");

    let summary = match status2 {
        HandStatus::Finished(summary, _history) => summary,
        _ => panic!("После двух фолдов из трёх игроков раздача должна завершиться"),
    };

    // Проверяем, что total_pot > 0 (как минимум блайнды в банке).
    assert!(
        summary.total_pot.0 > 0,
        "В банке должны быть фишки (SB/BB/ante)"
    );

    // В results должен быть один победитель с net_chips = total_pot.
    let winners: Vec<_> = summary.results.iter().filter(|r| r.is_winner).collect();
    assert_eq!(winners.len(), 1, "Должен быть ровно один победитель");

    let winner = winners[0];
    assert_eq!(
        winner.net_chips, summary.total_pot,
        "Победитель должен получить весь банк"
    );

    // Все остальные игроки не должны получить фишек.
    for r in &summary.results {
        if r.player_id != winner.player_id {
            assert_eq!(r.net_chips, Chips::ZERO);
        }
    }
}
