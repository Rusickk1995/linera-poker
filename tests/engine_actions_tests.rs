// tests/engine_actions_tests.rs

//! Тесты action-логики покерного движка:
//! - Call корректен (списывание стека, current_bet)
//! - Fold → игрок исчезает из to_act
//! - Bet увеличивает current_bet
//! - Raise обновляет min_raise
//! - All-in корректно работает

use poker_engine::domain::{
    blinds::AnteType,
    chips::Chips,
    player::{PlayerAtTable, PlayerStatus},
    table::{Table, TableConfig, TableStakes, TableType},
    HandId,
    PlayerId,
    TableId,
};
use poker_engine::engine::{
    apply_action,
    start_hand,
    actions::{PlayerAction, PlayerActionKind},
    HandStatus,
};
use poker_engine::infra::DeterministicRng;

/// Хелпер: создаёт простой турнирный стол на 2 игрока, с SB=50, BB=100, без анте.
fn make_two_player_table(initial_stack: Chips) -> Table {
    let table_id: TableId = 1;

    let stakes = TableStakes {
        small_blind: Chips(50),
        big_blind: Chips(100),
        ante: Chips::ZERO,
        ante_type: AnteType::None,
    };

    let config = TableConfig {
        max_seats: 9,
        table_type: TableType::Tournament,
        stakes,
        allow_straddle: false,
        allow_run_it_twice: false,
    };

    let mut table = Table::new(table_id, "Actions test table".to_string(), config);

    // Садим двух игроков на первые два места.
    table.seats[0] = Some(PlayerAtTable::new(1 as PlayerId, initial_stack));
    table.seats[1] = Some(PlayerAtTable::new(2 as PlayerId, initial_stack));

    table
}

/// Хелпер: возвращает (seat, player_id) текущего актёра.
use poker_engine::engine::game_loop::HandEngine;

fn current_actor_info(table: &Table, engine: &HandEngine) -> (u8, PlayerId) {
    let seat = engine.current_actor.expect("current_actor должен быть Some");
    let p = table.seats[seat as usize]
        .as_ref()
        .expect("seat должен быть занят");
    (seat, p.player_id)
}

//
// CALL
//

/// Call корректно списывает фишки и выравнивает current_bet.
#[test]
fn action_call_is_correct() {
    let initial_stack = Chips(10_000);
    let mut table = make_two_player_table(initial_stack);

    // Детерминированный RNG, чтобы не было рандомных падений.
    let mut rng = DeterministicRng::from_u64(42);

    // Стартуем раздачу.
    let hand_id: HandId = 1;
    let mut engine = start_hand(&mut table, &mut rng, hand_id).expect("start_hand failed");

    // Текущий актёр + его состояние до Call.
    let (seat, player_id) = current_actor_info(&table, &engine);
    let player_before = table.seats[seat as usize]
        .as_ref()
        .expect("player must be seated");
    let stack_before = player_before.stack;
    let bet_before = player_before.current_bet;

    // Сколько нужно докинуть до call.
    let to_call = if engine.betting.current_bet.0 > bet_before.0 {
        Chips(engine.betting.current_bet.0 - bet_before.0)
    } else {
        Chips::ZERO
    };

    // Делаем Call.
    let action = PlayerAction {
        seat,
        player_id,
        kind: PlayerActionKind::Call,
    };

    let status = apply_action(&mut table, &mut engine, action).expect("apply_action(Call) failed");

    // После Call раздача либо всё ещё идёт, либо уже завершилась.
    match status {
        HandStatus::Ongoing | HandStatus::Finished(_, _) => {}
    }

    let player_after = table.seats[seat as usize]
        .as_ref()
        .expect("player must still be seated");

    let stack_after = player_after.stack;
    let bet_after = player_after.current_bet;

    let paid = Chips(stack_before.0 - stack_after.0);

    // Игрок не может заплатить больше, чем нужно для call.
    assert!(
        paid.0 <= to_call.0,
        "paid={} must be <= to_call={}",
        paid.0,
        to_call.0
    );

    // Если денег хватало, мы ожидаем ровно to_call.
    if stack_before.0 > to_call.0 {
        assert_eq!(paid.0, to_call.0);
    }

    // Новый бет = старый + заплачено.
    assert_eq!(
        bet_after.0,
        bet_before.0 + paid.0,
        "current_bet must be increased by paid amount"
    );
}

//
// FOLD
//

/// Fold помечает игрока как Folded и убирает из очереди to_act.
#[test]
fn action_fold_removes_from_to_act() {
    let initial_stack = Chips(10_000);
    let mut table = make_two_player_table(initial_stack);
    let mut rng = DeterministicRng::from_u64(43);

    let hand_id: HandId = 2;
    let mut engine = start_hand(&mut table, &mut rng, hand_id).expect("start_hand failed");

    let (seat, player_id) = current_actor_info(&table, &engine);

    assert!(
        engine.betting.to_act.contains(&seat),
        "seat must be in to_act before Fold"
    );

    let action = PlayerAction {
        seat,
        player_id,
        kind: PlayerActionKind::Fold,
    };

    let _status = apply_action(&mut table, &mut engine, action).expect("apply_action(Fold) failed");

    let player_after = table.seats[seat as usize]
        .as_ref()
        .expect("player must remain seated (Fold does not remove seat)");

    assert!(
        matches!(player_after.status, PlayerStatus::Folded),
        "player status must be Folded after Fold action"
    );

    assert!(
        !engine.betting.to_act.contains(&seat),
        "seat must be removed from to_act after Fold"
    );
}

//
// BET
//

/// Bet на пустой улице (current_bet == 0) увеличивает current_bet и ставку игрока.
///
/// Мы вручную сбрасываем состояние ставок, чтобы смоделировать ситуацию
/// "первая ставка на улице" (например, на флопе).
#[test]
fn action_bet_increases_current_bet() {
    let initial_stack = Chips(10_000);
    let mut table = make_two_player_table(initial_stack);
    let mut rng = DeterministicRng::from_u64(44);

    let hand_id: HandId = 3;
    let mut engine = start_hand(&mut table, &mut rng, hand_id).expect("start_hand failed");

    // Делаем вид, что началась новая улица без предыдущих ставок.
    // Сбрасываем current_bet и очередность к одному игроку.
    let (seat, player_id) = current_actor_info(&table, &engine);

    // Сбрасываем беты игроков.
    for seat_opt in table.seats.iter_mut() {
        if let Some(p) = seat_opt {
            p.current_bet = Chips::ZERO;
        }
    }

    engine.betting.current_bet = Chips::ZERO;
    engine.betting.to_act.clear();
    engine.betting.to_act.push(seat);
    engine.current_actor = Some(seat);

    let bet_amount = Chips(300);

    let player_before = table.seats[seat as usize]
        .as_ref()
        .expect("player must exist")
        .clone();

    let action = PlayerAction {
        seat,
        player_id,
        kind: PlayerActionKind::Bet(bet_amount),
    };

    let _status = apply_action(&mut table, &mut engine, action).expect("apply_action(Bet) failed");

    let player_after = table.seats[seat as usize]
        .as_ref()
        .expect("player must exist after bet");

    // Проверяем, что фишки списались.
    assert!(
        player_after.stack.0 <= player_before.stack.0,
        "stack must not increase after Bet"
    );

    let paid = Chips(player_before.stack.0 - player_after.stack.0);

    // Сколько игрок реально поставил, то и его current_bet.
    assert_eq!(
        player_after.current_bet.0, paid.0,
        "player.current_bet must equal paid amount on first bet"
    );

    // current_bet в betting должен совпадать с размером бета.
    assert_eq!(
        engine.betting.current_bet.0, player_after.current_bet.0,
        "engine.betting.current_bet must match player's bet"
    );
}

//
// RAISE
//

/// Raise увеличивает current_bet и обновляет min_raise.
#[test]
fn action_raise_updates_min_raise() {
    let initial_stack = Chips(10_000);
    let mut table = make_two_player_table(initial_stack);
    let mut rng = DeterministicRng::from_u64(45);

    let hand_id: HandId = 4;
    let mut engine = start_hand(&mut table, &mut rng, hand_id).expect("start_hand failed");

    let (seat, player_id) = current_actor_info(&table, &engine);

    let old_current_bet = engine.betting.current_bet;
    let old_min_raise = engine.betting.min_raise;

    // Минимальный легальный рейз: current_bet + min_raise.
    let raise_to = Chips(old_current_bet.0 + old_min_raise.0);

    let player_before = table.seats[seat as usize]
        .as_ref()
        .expect("player must exist")
        .clone();

    let action = PlayerAction {
        seat,
        player_id,
        kind: PlayerActionKind::Raise(raise_to),
    };

    let _status =
        apply_action(&mut table, &mut engine, action).expect("apply_action(Raise) failed");

    let player_after = table.seats[seat as usize]
        .as_ref()
        .expect("player must exist after raise");

    // Текущий бет игрока должен совпадать с целевым.
    assert_eq!(
        player_after.current_bet, raise_to,
        "player.current_bet must equal target raise_to"
    );

    // Текущий bet в betting тоже.
    assert_eq!(
        engine.betting.current_bet, raise_to,
        "engine.betting.current_bet must equal raise_to"
    );

    // Новый размер минимального рейза = raise_to - old_current_bet.
    let expected_new_min_raise = Chips(raise_to.0 - old_current_bet.0);
    assert_eq!(
        engine.betting.min_raise, expected_new_min_raise,
        "min_raise must be updated to (raise_to - old_current_bet)"
    );

    // Стек уменьшился.
    assert!(
        player_after.stack.0 < player_before.stack.0,
        "stack must decrease after Raise"
    );
}

//
// ALL-IN
//

/// All-in выставляет игрока в статус AllIn, обнуляет стек и увеличивает current_bet.
#[test]
fn action_all_in_works() {
    let initial_stack = Chips(2_000);
    let mut table = make_two_player_table(initial_stack);
    let mut rng = DeterministicRng::from_u64(46);

    let hand_id: HandId = 5;
    let mut engine = start_hand(&mut table, &mut rng, hand_id).expect("start_hand failed");

    let (seat, player_id) = current_actor_info(&table, &engine);

    // Для простоты моделируем all-in как первый bet на улице:
    // сбрасываем ставки и выставляем current_actor на этого игрока.
    for seat_opt in table.seats.iter_mut() {
        if let Some(p) = seat_opt {
            p.current_bet = Chips::ZERO;
        }
    }
    engine.betting.current_bet = Chips::ZERO;
    engine.betting.to_act.clear();
    engine.betting.to_act.push(seat);
    engine.current_actor = Some(seat);

    let player_before = table.seats[seat as usize]
        .as_ref()
        .expect("player must exist")
        .clone();

    let action = PlayerAction {
        seat,
        player_id,
        kind: PlayerActionKind::AllIn,
    };

    let _status =
        apply_action(&mut table, &mut engine, action).expect("apply_action(AllIn) failed");

    let player_after = table.seats[seat as usize]
        .as_ref()
        .expect("player must exist");

    // Статус – AllIn.
    assert!(
        matches!(player_after.status, PlayerStatus::AllIn),
        "player status must be AllIn after AllIn action"
    );

    // Стек == 0.
    assert_eq!(
        player_after.stack.0, 0,
        "stack must become zero after AllIn"
    );

    // Игрок выложил в bet всё, что было.
    let invested = player_before.current_bet.0 + player_before.stack.0;
    assert_eq!(
        player_after.current_bet.0, invested,
        "current_bet must equal previous_bet + previous_stack on AllIn"
    );

    // Текущий bet в betting не меньше, чем ставка all-in игрока.
    assert!(
        engine.betting.current_bet.0 >= player_after.current_bet.0,
        "engine.betting.current_bet must be >= player's bet after AllIn"
    );
}
