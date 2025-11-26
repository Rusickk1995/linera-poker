// src/bin/poker_dev_cli_multitable.rs

use poker_engine::api::{build_table_view, TableViewDto};
use poker_engine::domain::blinds::AnteType;
use poker_engine::domain::chips::Chips;
use poker_engine::domain::hand::Street;
use poker_engine::domain::player::PlayerAtTable;
use poker_engine::domain::table::{Table, TableConfig, TableType, TableStakes};
use poker_engine::domain::{HandId, PlayerId, TableId, SeatIndex};
use poker_engine::engine::{
    HandStatus, PlayerAction, PlayerActionKind, TableManager, ManagerError,
};
use poker_engine::infra::{IdGenerator, SystemRng};

fn main() {
    println!("poker_dev_cli_multitable: стартуем мульти-табличный dev-CLI…");

    // 1. Инициализация генератора ID и RNG
    let mut id_gen = IdGenerator::new();
    let mut rng = SystemRng::default();

    // 2. Конфиг стола (общий для всех авто-столов)
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

    // 3. Менеджер столов
    let mut manager = TableManager::new();

    println!();
    println!("================ MULTI-TABLE SIMULATION =================");

    let mut extra_table_ids: Vec<TableId> = Vec::new();

    // Создадим 3 стола с разным количеством игроков (4–6).
    for n in 0..3 {
        let table_id = id_gen.next_table_id();
        let mut table = Table::new(
            table_id,
            format!("AUTO TABLE {}", n + 1),
            config.clone(),
        );

        // Число игроков: от 4 до 6
        let num_players = 4 + n; // 4, 5, 6
        for seat_index in 0..num_players {
            let pid = id_gen.next_player_id();
            table.seats[seat_index as usize] =
                Some(PlayerAtTable::new(pid, Chips::new(10_000)));
        }

        manager.add_table(table);
        extra_table_ids.push(table_id);
    }

    // На каждом из доп. столов сыграем по одной раздаче базовым сценарием.
    for table_id in extra_table_ids {
        println!();
        println!(
            "------ AUTO TABLE id={} | Single SimpleCheckCall hand ------",
            table_id
        );
        debug_print_table_state(&manager, table_id);

        play_hand(
            &mut manager,
            table_id,
            &mut rng,
            &mut id_gen,
            Scenario::SimpleCheckCall,
            "MULTI: SimpleCheckCall",
        );
    }

    println!("[CLI] Завершение работы dev-CLI (multitable).");
}

/// Сценарий тестовой раздачи.
#[derive(Copy, Clone, Debug)]
enum Scenario {
    SimpleCheckCall,
    WithFold,
    WithRaises,
    WithAllInSidePots,
}

/// Одна полная раздача по выбранному сценарию на заданном столе.
fn play_hand(
    manager: &mut TableManager,
    table_id: TableId,
    rng: &mut SystemRng,
    id_gen: &mut IdGenerator,
    scenario: Scenario,
    title: &str,
) {
    println!();
    println!("================ HAND {} =================", title);

    let hand_id: HandId = id_gen.next_hand_id();
    println!(
        "[CLI] Запускаем start_hand для table_id={}, hand_id={}.",
        table_id, hand_id
    );

    match manager.start_hand(table_id, rng, hand_id) {
        Ok(()) => {
            let dealer = manager
                .table(table_id)
                .and_then(|t| t.dealer_button)
                .unwrap_or(SeatIndex::from(0));
            println!(
                "[CLI] start_hand успешно отработал. Дилер на столе {} = Some({}).",
                table_id, dealer
            );
            debug_print_table_state(manager, table_id);
        }
        Err(e) => {
            println!(
                "[CLI] ОШИБКА в start_hand для стола {}: {:?}",
                table_id, e
            );
            debug_print_table_state(manager, table_id);
            println!("============ END HAND {} ============", title);
            return;
        }
    }

    if let Err(e) = run_scenario(manager, table_id, scenario) {
        println!(
            "[CLI] ОШИБКА в run_scenario для стола {}: {:?}",
            table_id, e
        );
    }

    println!("============ END HAND {} ============", title);
}

/// Прогон раздачи по выбранному сценарию, пока она не завершится.
fn run_scenario(
    manager: &mut TableManager,
    table_id: TableId,
    scenario: Scenario,
) -> Result<(), ManagerError> {
    const MAX_STEPS: u32 = 200;
    let mut step: u32 = 0;

    loop {
        step += 1;
        if step > MAX_STEPS {
            println!("[CLI] Превышен лимит шагов ({MAX_STEPS}), выходим.");
            break;
        }

        let seat = match manager.current_actor_seat(table_id) {
            Some(s) => s,
            None => {
                println!(
                    "[CLI] current_actor=None на столе {}, раздача, похоже, уже завершена логикой движка.",
                    table_id
                );
                break;
            }
        };

        let table_ref = match manager.table(table_id) {
            Some(t) => t,
            None => {
                println!("[CLI] BUG: стол {} не найден в менеджере.", table_id);
                break;
            }
        };

        let engine_ref = match manager.hand_engine(table_id) {
            Some(e) => e,
            None => {
                println!(
                    "[CLI] BUG: для стола {} нет активного HandEngine, хотя current_actor есть.",
                    table_id
                );
                break;
            }
        };

        let seat_idx = seat as usize;
        let player = match table_ref.seats.get(seat_idx).and_then(|s| s.as_ref()) {
            Some(p) => p,
            None => {
                println!(
                    "[CLI] BUG: current_actor указывает на пустое место seat={} на столе {}.",
                    seat, table_id
                );
                break;
            }
        };

        let action_kind =
            pick_scenario_action(scenario, step, table_ref, engine_ref, seat, player);

        let action = PlayerAction {
            player_id: player.player_id,
            seat,
            kind: action_kind.clone(),
        };

        println!(
            "[CLI][table_id={}] [step={}] street={:?} seat={} player_id={} -> {:?}",
            table_id,
            step,
            table_ref.street,
            seat,
            player.player_id,
            action_kind,
        );

        match manager.apply_action(table_id, action) {
            Err(e) => {
                println!(
                    "[CLI] ОШИБКА в apply_action на столе {}: {:?}",
                    table_id, e
                );
                debug_print_table_state(manager, table_id);
                return Err(e);
            }
            Ok(HandStatus::Ongoing) => {
                debug_print_table_state(manager, table_id);
            }
            Ok(HandStatus::Finished(summary, _history)) => {
                debug_print_table_state(manager, table_id);
                println!("=== РАЗДАЧА ЗАВЕРШЕНА ===");
                println!(
                    "table_id={} hand_id={} street_reached={:?} total_pot={}",
                    summary.table_id,
                    summary.hand_id,
                    summary.street_reached,
                    summary.total_pot.0
                );
                println!("Результаты игроков:");
                for r in summary.results {
                    println!(
                        "  player_id={} | net_chips={} | winner={}",
                        r.player_id,
                        r.net_chips.0,
                        r.is_winner
                    );
                }
                return Ok(());
            }
        }
    }

    Ok(())
}

/// Базовая стратегия бота (check/call/all-in/микро-bet постфлоп).
fn pick_base_action(
    table: &Table,
    engine: &poker_engine::engine::HandEngine,
    _seat: SeatIndex,
    player: &PlayerAtTable,
) -> PlayerActionKind {
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

/// Логика выбора действия в зависимости от сценария.
fn pick_scenario_action(
    scenario: Scenario,
    step: u32,
    table: &Table,
    engine: &poker_engine::engine::HandEngine,
    seat: SeatIndex,
    player: &PlayerAtTable,
) -> PlayerActionKind {
    match scenario {
        Scenario::SimpleCheckCall => pick_base_action(table, engine, seat, player),

        Scenario::WithFold => {
            if table.street == Street::Preflop {
                if step == 1 && seat == 1 {
                    return PlayerActionKind::Fold;
                }
                if step == 2 && seat == 2 {
                    return PlayerActionKind::Fold;
                }
            }
            pick_base_action(table, engine, seat, player)
        }

        Scenario::WithRaises => {
            if table.street == Street::Preflop && step == 1 && seat == 2 {
                return PlayerActionKind::Raise(Chips::new(300));
            }
            pick_base_action(table, engine, seat, player)
        }

        Scenario::WithAllInSidePots => {
            if table.street == Street::Preflop && step == 1 && seat == 0 {
                return PlayerActionKind::Raise(Chips::new(1_000));
            }
            pick_base_action(table, engine, seat, player)
        }
    }
}

// Печать состояния стола через API-слой (DTO).
fn debug_print_table_state(manager: &TableManager, table_id: TableId) {
    let table = match manager.table(table_id) {
        Some(t) => t,
        None => {
            println!(
                "[DEBUG] debug_print_table_state: стол {} не найден в менеджере.",
                table_id
            );
            return;
        }
    };

    let engine_opt = manager.hand_engine(table_id);

    let hero_id: PlayerId = table
        .seats
        .iter()
        .filter_map(|s| s.as_ref().map(|p| p.player_id))
        .next()
        .unwrap_or(0);

    let dto: TableViewDto = build_table_view(
        table,
        engine_opt,
        |pid: PlayerId| format!("P{}", pid),
        |pid: PlayerId| pid == hero_id,
    );

    let pot_for_display = if let Some(e) = engine_opt {
        e.pot.total.0
    } else {
        dto.total_pot.0
    };

    println!("================ TABLE STATE ================");
    println!(
        "table_id={} name={} street={:?} hand_in_progress={}",
        dto.table_id, dto.name, dto.street, dto.hand_in_progress
    );
    println!(
        "pot={} board={:?} dealer_button={:?} current_actor_seat={:?}",
        pot_for_display,
        dto.board,
        dto.dealer_button,
        dto.current_actor_seat,
    );
    println!("players:");
    for p in &dto.players {
        println!(
            "  seat {} | id={} | name={} | stack={} | bet={} | status={:?} | hole_cards={:?}",
            p.seat_index,
            p.player_id,
            p.display_name,
            p.stack.0,
            p.current_bet.0,
            p.status,
            p.hole_cards,
        );
    }
    println!("=============================================");
}
