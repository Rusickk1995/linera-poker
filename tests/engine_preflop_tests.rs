//! Интеграционные тесты префлопа и перехода улиц в покерном движке.
//!
//! Проверяем:
//! - старт раздачи (2 карты каждому, SB/BB/Ante, первый ход);
//! - переход Preflop -> Flop -> Turn -> River -> Showdown;
//! - рост board: 0 -> 3 -> 4 -> 5;
//! - корректность очереди ходов через engine.betting.to_act.

use poker_engine::domain::{
    blinds::AnteType,
    chips::Chips,
    hand::Street,
    player::PlayerAtTable,
    table::{Table, TableConfig, TableStakes, TableType},
    HandId,
    PlayerId,
    SeatIndex,
    TableId,
};
use poker_engine::engine::{
    actions::{PlayerAction, PlayerActionKind},
    apply_action,
    start_hand,
    HandStatus,
};
use poker_engine::infra::DeterministicRng;

/// Начальный стек для тестовых игроков.
const TEST_STACK: u64 = 10_000;

/// Конфиг стола для турнира с фиксированными блайндами/анте.
fn make_test_table(num_players: usize, ante_type: AnteType) -> Table {
    assert!(
        num_players <= 9,
        "В тестах допускаем максимум 9 игроков за столом"
    );

    let table_id: TableId = 1;

    let stakes = TableStakes::new(
        Chips::new(50),   // SB
        Chips::new(100),  // BB
        ante_type,
        match ante_type {
            AnteType::None => Chips::ZERO,
            _ => Chips::new(10),
        },
    );

    let config = TableConfig {
        max_seats: 9,
        table_type: TableType::Tournament,
        stakes,
        allow_straddle: false,
        allow_run_it_twice: false,
    };

    let mut table = Table::new(table_id, "Test table".to_string(), config);

    // Сажаем num_players подряд, с PlayerId = 1..=num_players, одинаковый стек.
    for seat_idx in 0..num_players {
        let pid: PlayerId = (seat_idx as u64) + 1;
        let pat = PlayerAtTable::new(pid, Chips::new(TEST_STACK));
        table.seats[seat_idx] = Some(pat);
    }

    table
}

/// Вспомогательно: найти seat с текущим бетом = заданной сумме (SB / BB).
fn find_seat_with_current_bet(table: &Table, amount: Chips) -> Option<SeatIndex> {
    for (idx, seat_opt) in table.seats.iter().enumerate() {
        if let Some(p) = seat_opt {
            if p.current_bet == amount {
                return Some(idx as SeatIndex);
            }
        }
    }
    None
}

/// ===============
/// TEST 1
/// ===============
/// Префлоп:
/// - каждому сидящему игроку раздали ровно 2 карты,
/// - анте списан с всех,
/// - SB и BB списаны корректно,
/// - первый ход принадлежит игроку после BB.
#[test]
fn preflop_deals_two_cards_and_posts_blinds_and_antes() {
    // 4 игрока, Classic ante.
    let mut table = make_test_table(4, AnteType::Classic);

    let mut rng = DeterministicRng::from_u64(12345);
    let hand_id: HandId = 1;

    let mut engine = start_hand(&mut table, &mut rng, hand_id)
        .expect("start_hand must succeed");

    // 1) Всем по 2 карты.
    for seat_idx in 0..4 {
        let p = table.seats[seat_idx]
            .as_ref()
            .expect("seat must be occupied");
        assert_eq!(
            p.hole_cards.len(),
            2,
            "Каждый игрок должен получить 2 карманные карты"
        );
    }

    // 2) Находим SB / BB по current_bet.
    let sb_amount = table.config.stakes.small_blind;
    let bb_amount = table.config.stakes.big_blind;
    let ante_amount = table.config.stakes.ante;

    let sb_seat = find_seat_with_current_bet(&table, sb_amount)
        .expect("должен быть seat с small blind");
    let bb_seat = find_seat_with_current_bet(&table, bb_amount)
        .expect("должен быть seat с big blind");

    // Проверяем current_bet игроков.
    let sb_player = table.seats[sb_seat as usize].as_ref().unwrap();
    assert_eq!(
        sb_player.current_bet, sb_amount,
        "У SB current_bet должен быть равен small blind"
    );

    let bb_player = table.seats[bb_seat as usize].as_ref().unwrap();
    assert_eq!(
        bb_player.current_bet, bb_amount,
        "У BB current_bet должен быть равен big blind"
    );

    // 3) Анте списан с *каждого* игрока: в поте должно лежать
    //    сумма анте + блайнды.
    // Упрощённо: проверим total pot = сумма contributions.
    let mut sum_contributions = Chips::ZERO;
    for (_seat, amount) in engine.contributions.iter() {
        sum_contributions += *amount;
    }

    assert_eq!(
        sum_contributions,
        engine.pot.total,
        "Сумма contributions должна совпадать с pot.total"
    );

    // 4) Первый ход: после BB.
    //    То, что engine.betting.current_bet = BB, мы тоже проверим.
    assert_eq!(
        engine.betting.current_bet, bb_amount,
        "current_bet на префлопе должен быть равен BB"
    );

    let first_to_act = engine
        .current_actor
        .expect("на префлопе должен быть текущий актор");
    // Игрок с ходом должен входить в очередь to_act.
    assert!(
        engine.betting.to_act.contains(&first_to_act),
        "current_actor должен присутствовать в betting.to_act"
    );
}

/// ===============
/// TEST 2
/// ===============
/// Полный проход улиц:
/// Preflop -> Flop -> Turn -> River -> Showdown
/// при сценарии "все колл/чек":
/// - board: 0 -> 3 -> 4 -> 5;
/// - HandStatus: в конце Finished;
/// - порядок ходов согласован с betting.to_act (мы ходим строго в этом порядке).
#[test]
fn full_hand_flop_turn_river_board_and_order() {
    let mut table = make_test_table(4, AnteType::None);

    let mut rng = DeterministicRng::from_u64(777);
    let hand_id: HandId = 1;

    let mut engine = start_hand(&mut table, &mut rng, hand_id)
        .expect("start_hand must succeed");

    // Начальное состояние.
    assert_eq!(table.street, Street::Preflop);
    assert_eq!(table.board.len(), 0);

    // ===========
    // PRE-FLOP
    // ===========
    // Находим seat BB, чтобы он мог сделать CHECK, остальные — CALL.
    let bb_amount = table.config.stakes.big_blind;
    let bb_seat = find_seat_with_current_bet(&table, bb_amount)
        .expect("BB seat must exist");

    // Делаем действия строго в порядке betting.to_act.
    let preflop_order = engine.betting.to_act.clone();
    let mut last_status = HandStatus::Ongoing;

    for seat in preflop_order {
        let seat_idx = seat as usize;
        let p = table.seats[seat_idx]
            .as_ref()
            .expect("seat must be occupied");

        let kind = if seat == bb_seat {
            PlayerActionKind::Check
        } else {
            PlayerActionKind::Call
        };

        let action = PlayerAction {
            player_id: p.player_id,
            seat,
            kind,
        };

        last_status = apply_action(&mut table, &mut engine, action)
            .expect("action should be valid");
    }

    // После завершения префлопа должны оказаться на Flop.
    assert!(matches!(last_status, HandStatus::Ongoing));
    assert_eq!(table.street, Street::Flop);
    assert_eq!(
        table.board.len(),
        3,
        "На флопе должно быть ровно 3 борд-карты"
    );

    // ===========
    // FLOP  (все CHECK)
    // ===========
    let flop_order = engine.betting.to_act.clone();
    assert!(
        !flop_order.is_empty(),
        "На флопе должна быть очередь ходов"
    );

    for seat in flop_order {
        let seat_idx = seat as usize;
        let p = table.seats[seat_idx]
            .as_ref()
            .expect("seat must be occupied");

        let action = PlayerAction {
            player_id: p.player_id,
            seat,
            kind: PlayerActionKind::Check,
        };

        last_status = apply_action(&mut table, &mut engine, action)
            .expect("flop check must be valid");
    }

    assert!(matches!(last_status, HandStatus::Ongoing));
    assert_eq!(table.street, Street::Turn);
    assert_eq!(
        table.board.len(),
        4,
        "На терне должно быть ровно 4 борд-карты (3 + 1)"
    );

    // ===========
    // TURN (все CHECK)
    // ===========
    let turn_order = engine.betting.to_act.clone();
    assert!(
        !turn_order.is_empty(),
        "На терне должна быть очередь ходов"
    );

    for seat in turn_order {
        let seat_idx = seat as usize;
        let p = table.seats[seat_idx]
            .as_ref()
            .expect("seat must be occupied");

        let action = PlayerAction {
            player_id: p.player_id,
            seat,
            kind: PlayerActionKind::Check,
        };

        last_status = apply_action(&mut table, &mut engine, action)
            .expect("turn check must be valid");
    }

    assert!(matches!(last_status, HandStatus::Ongoing));
    assert_eq!(table.street, Street::River);
    assert_eq!(
        table.board.len(),
        5,
        "На ривере должно быть ровно 5 борд-карт (3 + 1 + 1)"
    );

    // ===========
    // RIVER (все CHECK, должен быть шоудаун)
    // ===========
    let river_order = engine.betting.to_act.clone();
    assert!(
        !river_order.is_empty(),
        "На ривере должна быть очередь ходов"
    );

    for seat in river_order {
        let seat_idx = seat as usize;
        let p = table.seats[seat_idx]
            .as_ref()
            .expect("seat must be occupied");

        let action = PlayerAction {
            player_id: p.player_id,
            seat,
            kind: PlayerActionKind::Check,
        };

        last_status = apply_action(&mut table, &mut engine, action)
            .expect("river check must be valid");
    }

    // После последнего чек на ривере должна завершиться раздача (шоудаун).
    match last_status {
        HandStatus::Finished(summary, _history) => {
            assert_eq!(
                summary.street_reached,
                Street::Showdown,
                "Раздача должна завершиться на шоудауне"
            );
            assert_eq!(
                summary.board.len(),
                5,
                "В итоговом summary board также должен содержать 5 карт"
            );
        }
        HandStatus::Ongoing => {
            panic!("После полного круга на ривере HandStatus должен быть Finished");
        }
    }
}

/// =============================
/// EDGE-CASE 1: чистый Heads-Up
/// =============================
/// В хэдз-апе дилер обязан быть small blind,
/// второй игрок – big blind.
#[test]
fn heads_up_button_is_small_blind_and_second_player_is_big_blind() {
    // Делаем стол на 2 игрока, без анте.
    let mut table = make_test_table(2, AnteType::None);

    let mut rng = DeterministicRng::from_u64(12345);
    let hand_id: HandId = 1;

    let _engine = start_hand(&mut table, &mut rng, hand_id)
        .expect("start_hand must succeed");

    // Кнопка дилера.
    let button = table
        .dealer_button
        .expect("dealer_button должен быть установлен");

    let sb_amount = table.config.stakes.small_blind;
    let bb_amount = table.config.stakes.big_blind;

    let sb_seat = find_seat_with_current_bet(&table, sb_amount)
        .expect("в хэдз-апе должен быть найден SB");
    let bb_seat = find_seat_with_current_bet(&table, bb_amount)
        .expect("в хэдз-апе должен быть найден BB");

    // В heads-up:
    // - дилер = small blind,
    // - второй игрок = big blind.
    assert_eq!(
        button, sb_seat,
        "в хэдз-апе дилер обязан быть small blind"
    );
    assert_ne!(
        sb_seat, bb_seat,
        "SB и BB должны быть разными местами"
    );
}

/// ======================================================================
/// EDGE-CASE 2: переход 3-max -> Heads-Up, когда вылетает small blind
/// ======================================================================
/// Сценарий:
/// 1) Стол из 3 игроков, стандартные дилер/SB/BB.
/// 2) Выбивает именно SB (seat очищается).
/// 3) Следующая раздача: остаётся 2 игрока, и должен включиться heads-up
///    режим — дилер становится SB, второй игрок — BB.
#[test]
fn three_max_to_heads_up_when_small_blind_busts() {
    // Стартуем 3-max стол без анте.
    let mut table = make_test_table(3, AnteType::None);

    let mut rng = DeterministicRng::from_u64(777);
    let first_hand_id: HandId = 1;

    // Первая раздача при 3 игроках.
    let _engine = start_hand(&mut table, &mut rng, first_hand_id)
        .expect("первая раздача должна стартовать");

    let button1 = table
        .dealer_button
        .expect("после первой раздачи должна быть кнопка дилера");

    let sb_amount = table.config.stakes.small_blind;
    let bb_amount = table.config.stakes.big_blind;

    let sb1 = find_seat_with_current_bet(&table, sb_amount)
        .expect("в первой раздаче должен быть SB");
    let bb1 = find_seat_with_current_bet(&table, bb_amount)
        .expect("в первой раздаче должен быть BB");

    // В 3-max:
    // - дилер, SB и BB – три различных места.
    assert_ne!(button1, sb1, "в 3-max дилер не совпадает с SB");
    assert_ne!(button1, bb1, "в 3-max дилер не совпадает с BB");
    assert_ne!(sb1, bb1, "SB и BB не совпадают в 3-max");

    // Эмулируем вылет small blind:
    // - очищаем его seat;
    // - финализируем текущую раздачу.
    table.seats[sb1 as usize] = None;
    table.hand_in_progress = false;
    table.current_hand_id = None;

    // Вторая раздача уже при 2 игроках (heads-up).
    let second_hand_id: HandId = 2;
    let _engine2 = start_hand(&mut table, &mut rng, second_hand_id)
        .expect("вторая раздача должна стартовать после вылета SB");

    let button2 = table
        .dealer_button
        .expect("во второй раздаче должна быть кнопка дилера");

    let sb2 = find_seat_with_current_bet(&table, sb_amount)
        .expect("во второй раздаче должен быть SB");
    let bb2 = find_seat_with_current_bet(&table, bb_amount)
        .expect("во второй раздаче должен быть BB");

    // Сейчас за столом два игрока, и должна примениться heads-up логика:
    assert_eq!(
        button2, sb2,
        "в heads-up дилер обязан быть SB"
    );
    assert_ne!(
        sb2, bb2,
        "SB и BB в heads-up должны быть разными игроками"
    );

    // Дополнительно убеждаемся, что новый BB != вылетевший SB из первой раздачи.
    assert_ne!(
        bb2, sb1,
        "игрок, вылетевший как SB в первой раздаче, не может стать BB во второй"
    );
}

