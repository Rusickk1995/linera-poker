use poker_engine::domain::blinds::AnteType;
use poker_engine::domain::chips::Chips;
use poker_engine::domain::player::PlayerAtTable;
use poker_engine::domain::table::{Table, TableConfig, TableStakes, TableType};
use poker_engine::domain::{HandId, PlayerId, SeatIndex, TableId};
use poker_engine::engine::{HandStatus, PlayerAction, PlayerActionKind, TableManager};
use poker_engine::infra::{IdGenerator, SystemRng};

fn main() {
    println!("poker_stress_test: стартуем стресс-тест покерного движка…");

    // Параметры нагрузки — можно смело крутить.
    const NUM_TABLES: usize = 32;        // сколько столов
    const PLAYERS_PER_TABLE: usize = 6;  // игроков за столом
    const HANDS_PER_TABLE: u32 = 200;    // сколько раздач на стол

    let mut id_gen = IdGenerator::new();
    let mut rng = SystemRng::default();
    let mut manager = TableManager::new();

    // Конфиг стола: 50/100, без анте.
    let stakes = TableStakes::new(
        Chips::new(50),
        Chips::new(100),
        AnteType::None,
        Chips::ZERO,
    );

    let config = TableConfig {
        max_seats: 9,
        table_type: TableType::Cash,
        stakes,
        allow_straddle: false,
        allow_run_it_twice: false,
    };

    // 1. Создаём много столов и рассаживаем игроков.
    let mut table_ids: Vec<TableId> = Vec::with_capacity(NUM_TABLES);

    for t in 0..NUM_TABLES {
        let table_id: TableId = id_gen.next_table_id();
        let mut table = Table::new(
            table_id,
            format!("STRESS TABLE {}", t + 1),
            config.clone(),
        );

        for seat_idx in 0..PLAYERS_PER_TABLE {
            let pid: PlayerId = id_gen.next_player_id();
            table.seats[seat_idx] = Some(PlayerAtTable::new(pid, Chips::new(10_000)));
        }

        manager.add_table(table);
        table_ids.push(table_id);
    }

    println!(
        "[STRESS] Создано {} столов, по {} игроков, по {} рук на стол.",
        NUM_TABLES, PLAYERS_PER_TABLE, HANDS_PER_TABLE
    );

    // Статистика.
    let mut total_hands: u64 = 0;
    let mut total_pot: u64 = 0;
    let mut max_pot: u64 = 0;
    let mut num_showdowns: u64 = 0;

    // 2. Гоним раздачи по всем столам.
    for &table_id in &table_ids {
        for _ in 0..HANDS_PER_TABLE {
            let hand_id: HandId = id_gen.next_hand_id();

            if let Err(e) = manager.start_hand(table_id, &mut rng, hand_id) {
                eprintln!(
                    "[STRESS][table_id={}] ОШИБКА в start_hand: {:?}",
                    table_id, e
                );
                break;
            }

            match play_single_hand_stress(&mut manager, table_id) {
                Ok(Some(stats)) => {
                    total_hands += 1;
                    total_pot += stats.total_pot;

                    if stats.total_pot > max_pot {
                        max_pot = stats.total_pot;
                    }
                    if stats.reached_showdown {
                        num_showdowns += 1;
                    }
                }
                Ok(None) => {
                    // Раздача не дошла до Finished по какой-то причине (не должно происходить).
                    eprintln!(
                        "[STRESS][table_id={}] hand_id={:?}: раздача не завершилась корректно",
                        table_id, hand_id
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[STRESS][table_id={}] hand_id={:?}: ОШИБКА в ходе раздачи: {:?}",
                        table_id, hand_id, e
                    );
                }
            }
        }
    }

    println!();
    println!("=========== STRESS TEST SUMMARY ===========");
    println!("Всего сыграно рук: {}", total_hands);
    if total_hands > 0 {
        let avg_pot = total_pot / total_hands;
        println!("Суммарный пот за все руки: {}", total_pot);
        println!("Средний пот: {}", avg_pot);
        println!("Максимальный пот: {}", max_pot);
        println!("Рук дошло до шоудауна: {}", num_showdowns);
    }
    println!("===========================================");
    println!("poker_stress_test: завершено.");
}

/// Итог одной раздачи, который нам нужен для статистики.
struct HandStats {
    total_pot: u64,
    reached_showdown: bool,
}

/// Прогон одной раздачи для стресс-теста:
/// - без детальной печати стола;
/// - простая бот-логика "check/call/bet 1BB".
fn play_single_hand_stress(
    manager: &mut TableManager,
    table_id: TableId,
) -> Result<Option<HandStats>, poker_engine::engine::ManagerError> {
    use poker_engine::domain::hand::Street;
    use poker_engine::domain::table::Table as TableDomain;
    use poker_engine::domain::player::PlayerAtTable as PlayerDomain;

    const MAX_STEPS: u32 = 200;
    let mut step: u32 = 0;

    loop {
        step += 1;
        if step > MAX_STEPS {
            eprintln!(
                "[STRESS][table_id={}] превышен лимит шагов ({MAX_STEPS}), выходим из раздачи",
                table_id
            );
            return Ok(None);
        }

        let seat = match manager.current_actor_seat(table_id) {
            Some(s) => s,
            None => {
                // current_actor отсутствует — раздача уже должна была завершиться логикой движка.
                return Ok(None);
            }
        };

        let table_ref: &TableDomain = match manager.table(table_id) {
            Some(t) => t,
            None => {
                eprintln!(
                    "[STRESS][table_id={}] BUG: стол не найден при активном актёре.",
                    table_id
                );
                return Ok(None);
            }
        };

        let engine_ref = match manager.hand_engine(table_id) {
            Some(e) => e,
            None => {
                eprintln!(
                    "[STRESS][table_id={}] BUG: нет HandEngine при наличии current_actor.",
                    table_id
                );
                return Ok(None);
            }
        };

        let seat_idx = seat as usize;
        let player: &PlayerDomain = match table_ref.seats.get(seat_idx).and_then(|s| s.as_ref()) {
            Some(p) => p,
            None => {
                eprintln!(
                    "[STRESS][table_id={}] BUG: current_actor указывает на пустое место seat={}.",
                    table_id, seat
                );
                return Ok(None);
            }
        };

        let action_kind = pick_base_action_stress(table_ref, engine_ref, seat, player);
        let action = PlayerAction {
            player_id: player.player_id,
            seat,
            kind: action_kind,
        };

        match manager.apply_action(table_id, action)? {
            HandStatus::Ongoing => {
                // продолжаем цикл
            }
            HandStatus::Finished(summary, _history) => {
                let stats = HandStats {
                    total_pot: summary.total_pot.0,
                    reached_showdown: matches!(summary.street_reached, Street::Showdown),
                };
                return Ok(Some(stats));
            }
        }
    }
}

/// Простейшая стратегия для стресс-теста:
/// - если нечего доплачивать:
///   * префлоп → Check
///   * постфлоп → ставим 1 BB (или min_raise), если есть стек
/// - если нужно доплатить:
///   * если не хватает стека → AllIn
///   * иначе → Call
fn pick_base_action_stress(
    table: &Table,
    engine: &poker_engine::engine::HandEngine,
    _seat: SeatIndex,
    player: &PlayerAtTable,
) -> PlayerActionKind {
    use poker_engine::domain::hand::Street;

    let current_bet = engine.betting.current_bet;
    let player_bet = player.current_bet;

    let to_call_amount = if current_bet.0 > player_bet.0 {
        current_bet.0 - player_bet.0
    } else {
        0
    };

    if to_call_amount == 0 {
        match table.street {
            Street::Preflop => PlayerActionKind::Check,
            Street::Flop | Street::Turn | Street::River => {
                let stake_bb = table.config.stakes.big_blind;
                let min_bet = if engine.betting.min_raise.0 > stake_bb.0 {
                    engine.betting.min_raise
                } else {
                    stake_bb
                };

                if player.stack.0 == 0 || min_bet.0 == 0 {
                    PlayerActionKind::Check
                } else {
                    PlayerActionKind::Bet(min_bet)
                }
            }
            Street::Showdown => PlayerActionKind::Check,
        }
    } else {
        if player.stack.0 <= to_call_amount {
            PlayerActionKind::AllIn
        } else {
            PlayerActionKind::Call
        }
    }
}
