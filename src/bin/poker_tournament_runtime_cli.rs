// src/bin/poker_tournament_runtime_cli.rs
//
// Полноценная симуляция турнирного покера:
// - TournamentLobby + Tournament
// - Seating по уровням
// - Реальные столы Table
// - Реальные раздачи через HandEngine + TableManager
// - Боты (4 профиля)
// - Вылеты по stack==0
// - Пересаживание на каждом уровне
// - Финалка + победитель (доигрываем ДО 1 игрока)
//

use poker_engine::domain::blinds::AnteType;
use poker_engine::domain::chips::Chips;
use poker_engine::domain::hand::Street;
use poker_engine::domain::player::PlayerAtTable;
use poker_engine::domain::table::{Table, TableConfig, TableStakes, TableType};
use poker_engine::domain::tournament::{Tournament, TournamentConfig, TournamentStatus};
use poker_engine::domain::{PlayerId, SeatIndex, TableId, TournamentId};
use poker_engine::engine::{
    HandStatus, ManagerError, PlayerAction, PlayerActionKind, TableManager,
};
use poker_engine::infra::{IdGenerator, SystemRng};
use poker_engine::time_ctrl::{AutoActionDecision, TimeController, TimeRules};
use poker_engine::tournament::TournamentLobby;

// ======= ПРОФИЛИ БОТОВ =====================================================

#[derive(Clone, Copy, Debug)]
enum TableProfile {
    TightPassive,
    LooseAggressive,
    PushOrFold,
    Mixed,
}

#[derive(Default)]
struct RuntimeStats {
    hands_planned: u64,
    hands_finished: u64,
    hands_finished_no_actor: u64,
    hands_aborted: u64,
}

#[derive(Clone, Copy, Debug)]
enum HandResult {
    FinishedNormal,
    FinishedNoActorEngineAlive,
}

const HANDS_PER_LEVEL: u32 = 6;
const MAX_STEPS_PER_HAND: u32 = 180;

// ====== BOT LOGIC ==========================================================

fn to_call(engine: &poker_engine::engine::HandEngine, p: &PlayerAtTable) -> u64 {
    let cb = engine.betting.current_bet.0;
    let pb = p.current_bet.0;
    cb.saturating_sub(pb)
}

fn make_all_in(engine: &poker_engine::engine::HandEngine, p: &PlayerAtTable) -> PlayerActionKind {
    let cb = engine.betting.current_bet.0;
    let mr = engine.betting.min_raise.0;
    let total = p.current_bet.0 + p.stack.0;

    if total <= cb {
        return PlayerActionKind::Call;
    }

    if cb == 0 {
        return PlayerActionKind::Bet(Chips::new(total));
    }

    let extra = total - cb;
    if mr > 0 && extra < mr {
        PlayerActionKind::Call
    } else {
        PlayerActionKind::Raise(Chips::new(total))
    }
}

fn pick_action(
    profile: TableProfile,
    h: u32,
    step: u32,
    table: &Table,
    eng: &poker_engine::engine::HandEngine,
    seat: SeatIndex,
    p: &PlayerAtTable,
) -> PlayerActionKind {
    let pattern = (h + step + seat as u32) % 10;
    let bb = table.config.stakes.big_blind.0.max(1);
    let stack = p.stack.0;

    // ===============================
    // 1. Helper’ы нормализации ставок
    // ===============================
    let safe_raise = |target: u64| -> PlayerActionKind {
        let cb = eng.betting.current_bet.0;
        let mr = eng.betting.min_raise.0;

        // Нельзя рейзить меньше, чем текущий бет + min_raise
        if mr > 0 && target < cb + mr {
            return PlayerActionKind::Call;
        }
        // Нельзя ставить больше, чем можем внести (текущая ставка + стек)
        if target > p.current_bet.0 + p.stack.0 {
            return PlayerActionKind::Call;
        }

        PlayerActionKind::Raise(Chips::new(target))
    };

    let safe_bet = |target: u64| -> PlayerActionKind {
        let bet_size = target.min(stack);
        if bet_size == 0 {
            PlayerActionKind::Check
        } else {
            PlayerActionKind::Bet(Chips::new(bet_size))
        }
    };

    let call_amt = {
        let cb = eng.betting.current_bet.0;
        let pb = p.current_bet.0;
        cb.saturating_sub(pb)
    };

    if stack == 0 {
        return if call_amt > 0 {
            PlayerActionKind::Fold
        } else {
            PlayerActionKind::Check
        };
    }

    // шансы пуша при коротком стеке
    if stack <= 8 * bb {
        let shove = match profile {
            TableProfile::TightPassive => 0,
            TableProfile::PushOrFold => 7,
            TableProfile::Mixed => 4,
            TableProfile::LooseAggressive => 5,
        };
        if pattern < shove {
            return make_all_in(eng, p);
        }
    }

    // Есть ставка для call
    if call_amt > 0 {
        if stack < call_amt {
            return PlayerActionKind::Fold;
        }

        let agr = match profile {
            TableProfile::TightPassive => 2,
            TableProfile::PushOrFold => 3,
            TableProfile::Mixed => 4,
            TableProfile::LooseAggressive => 6,
        };

        if pattern < agr {
            let cb = eng.betting.current_bet.0;
            let mr = eng.betting.min_raise.0;
            let target = cb + mr;
            return safe_raise(target);
        }

        return PlayerActionKind::Call;
    }

    // Открываем торги (нет call_amt)
    match table.street {
        Street::Preflop => {
            let agr = match profile {
                TableProfile::TightPassive => 3,
                TableProfile::PushOrFold => 4,
                TableProfile::Mixed => 5,
                TableProfile::LooseAggressive => 7,
            };
            if pattern < agr {
                return safe_bet(2 * bb);
            }
            PlayerActionKind::Check
        }
        Street::Flop | Street::Turn | Street::River | Street::Showdown => {
            let agr = match profile {
                TableProfile::TightPassive => 4,
                TableProfile::PushOrFold => 3,
                TableProfile::Mixed => 2,
                TableProfile::LooseAggressive => 1,
            };

            if pattern < agr {
                let cb = eng.betting.current_bet.0;
                if cb == 0 {
                    // ставим около 2bb, но через safe_bet
                    return safe_bet(2 * bb);
                } else {
                    let target = cb + eng.betting.min_raise.0;
                    return safe_raise(target);
                }
            }

            // пассивная линия
            if call_amt > 0 {
                if stack >= call_amt {
                    PlayerActionKind::Call
                } else {
                    PlayerActionKind::Fold
                }
            } else {
                PlayerActionKind::Check
            }
        }
    }
}

// ====== ОДНА РУКА ==========================================================

fn play_hand(
    mgr: &mut TableManager,
    table_id: TableId,
    profile: TableProfile,
    h: u32,
    stats: &mut RuntimeStats,
    time_ctrl: &mut TimeController,
) -> Result<HandResult, ()> {
    let mut step = 0;

    loop {
        step += 1;
        if step > MAX_STEPS_PER_HAND {
            println!(
                "[TOURNAMENT][table_id={}] hand_seq={} превышен лимит шагов ({}) — вероятный баг логики.",
                table_id, h, MAX_STEPS_PER_HAND
            );
            stats.hands_aborted += 1;
            return Err(());
        }

        let engine_exists = mgr.hand_engine(table_id).is_some();
        let seat_opt = mgr.current_actor_seat(table_id);

        match (engine_exists, seat_opt) {
            // Нет HandEngine и нет текущего актёра — рука завершена (нормальный кейс).
            (false, None) => {
                stats.hands_finished += 1;
                return Ok(HandResult::FinishedNormal);
            }
            // Есть HandEngine и есть актёр — нормальная работа.
            (true, Some(seat)) => {
                // 1) выбираем действие БЕЗ удержания ссылок на mgr во время apply_action
                let (player_id, action_kind) = {
                    let table_ref = match mgr.table(table_id) {
                        Some(t) => t,
                        None => {
                            println!(
                                "[TOURNAMENT][table_id={}] hand_seq={} BUG: стол исчез из менеджера.",
                                table_id, h
                            );
                            stats.hands_aborted += 1;
                            return Err(());
                        }
                    };

                    let engine_ref = match mgr.hand_engine(table_id) {
                        Some(e) => e,
                        None => {
                            println!(
                                "[TOURNAMENT][table_id={}] hand_seq={} BUG: есть current_actor, но нет HandEngine (гонка состояний).",
                                table_id, h
                            );
                            stats.hands_aborted += 1;
                            return Err(());
                        }
                    };

                    let seat_idx = seat as usize;
                    let player = match table_ref.seats.get(seat_idx).and_then(|s| s.as_ref()) {
                        Some(p) => p,
                        None => {
                            println!(
                                "[TOURNAMENT][table_id={}] hand_seq={} BUG: current_actor указывает на пустое место seat={}.",
                                table_id, h, seat
                            );
                            stats.hands_aborted += 1;
                            return Err(());
                        }
                    };

                    // ----- ВРЕМЯ НА ХОД -------------------------------------
                    // Начинаем ход для этого игрока по стандартным правилам (20 сек + банк).
                    time_ctrl.start_turn(player.player_id);

                    // Симулируем "подумал N секунд" (1..=5).
                    let think_secs =
                        ((h + step + seat as u32) % 5 + 1) as i32;

                    let auto_decision = time_ctrl.on_time_passed(think_secs);

                    let action_kind = match auto_decision {
                        AutoActionDecision::None => {
                            // Игрок (бот) успел — выбираем нормальное действие.
                            pick_action(profile, h, step, table_ref, engine_ref, seat, player)
                        }
                        AutoActionDecision::TimeoutCheckOrFold => {
                            // Полный таймаут: AUTO CHECK / AUTO FOLD.
                            let call_amt = to_call(engine_ref, player);
                            if call_amt > 0 {
                                // Нельзя бесплатно чекнуть — автосклад.
                                PlayerActionKind::Fold
                            } else {
                                // Можно чекнуть — авто-check.
                                PlayerActionKind::Check
                            }
                        }
                    };

                    (player.player_id, action_kind)
                };

                let action = PlayerAction {
                    player_id,
                    seat,
                    kind: action_kind,
                };

                // 2) пытаемся применить действие
                match mgr.apply_action(table_id, action) {
                    Ok(HandStatus::Ongoing) => {
                        // действие прошло, очищаем таймер для этого игрока
                        time_ctrl.on_manual_action(player_id);
                    }
                    Ok(HandStatus::Finished(_, _)) => {
                        time_ctrl.on_manual_action(player_id);
                        stats.hands_finished += 1;
                        return Ok(HandResult::FinishedNormal);
                    }
                    Err(ManagerError::Engine(e)) => {
                        // FALLBACK на безопасное действие
                        println!(
                            "[TOURNAMENT][table_id={}] hand_seq={} step={} ILLEGAL ⇒ FALLBACK: {:?}",
                            table_id, h, step, e
                        );

                        // заново берём engine и игрока для расчёта безопасного действия
                        let (player_for_fb, engine_for_fb) = {
                            let table_ref = match mgr.table(table_id) {
                                Some(t) => t,
                                None => {
                                    println!(
                                        "[TOURNAMENT][table_id={}] hand_seq={} FALLBACK BUG: стол исчез из менеджера.",
                                        table_id, h
                                    );
                                    stats.hands_aborted += 1;
                                    return Err(());
                                }
                            };
                            let engine_ref = match mgr.hand_engine(table_id) {
                                Some(e) => e,
                                None => {
                                    println!(
                                        "[TOURNAMENT][table_id={}] hand_seq={} FALLBACK BUG: нет HandEngine.",
                                        table_id, h
                                    );
                                    stats.hands_aborted += 1;
                                    return Err(());
                                }
                            };
                            let seat_idx = seat as usize;
                            let player = match table_ref.seats.get(seat_idx).and_then(|s| s.as_ref())
                            {
                                Some(p) => p,
                                None => {
                                    println!(
                                        "[TOURNAMENT][table_id={}] hand_seq={} FALLBACK BUG: пустое место seat={}.",
                                        table_id, h, seat
                                    );
                                    stats.hands_aborted += 1;
                                    return Err(());
                                }
                            };
                            (player.clone(), engine_ref)
                        };

                        let call_amt_fb = to_call(engine_for_fb, &player_for_fb);
                        let fallback_kind = if call_amt_fb > 0 {
                            // для фоллбэка в случае ошибки движка логичнее тоже играть
                            // безопасно: не коллить автоматически, а либо фолд, либо чек.
                            if player_for_fb.stack.0 >= call_amt_fb {
                                PlayerActionKind::Fold
                            } else {
                                PlayerActionKind::Fold
                            }
                        } else {
                            PlayerActionKind::Check
                        };

                        let fallback_action = PlayerAction {
                            player_id,
                            seat,
                            kind: fallback_kind,
                        };

                        match mgr.apply_action(table_id, fallback_action) {
                            Ok(HandStatus::Ongoing) => {
                                time_ctrl.on_manual_action(player_id);
                                // продолжаем цикл руки
                                continue;
                            }
                            Ok(HandStatus::Finished(_, _)) => {
                                time_ctrl.on_manual_action(player_id);
                                stats.hands_finished += 1;
                                return Ok(HandResult::FinishedNormal);
                            }
                            Err(err2) => {
                                println!(
                                    "[TOURNAMENT][table_id={}] hand_seq={} step={} FALLBACK FAILED: {:?}",
                                    table_id, h, step, err2
                                );
                                stats.hands_aborted += 1;
                                return Err(());
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "[TOURNAMENT][table_id={}] hand_seq={} step={} ОШИБКА менеджера: {:?}",
                            table_id, h, step, e
                        );
                        stats.hands_aborted += 1;
                        return Err(());
                    }
                }
            }
            // Есть HandEngine, но нет current_actor — считаем, что рука завершена мгновенно.
            (true, None) => {
                println!(
                    "[TOURNAMENT][table_id={}] hand_seq={} INFO: HandEngine жив, но актёра нет — считаем руку завершённой (крайний кейс).",
                    table_id, h
                );
                stats.hands_finished += 1;
                stats.hands_finished_no_actor += 1;
                return Ok(HandResult::FinishedNoActorEngineAlive);
            }
            // Нет HandEngine, но есть current_actor — это реальный баг.
            (false, Some(seat)) => {
                println!(
                    "[TOURNAMENT][table_id={}] hand_seq={} BUG: нет HandEngine, но current_actor_seat = {:?}.",
                    table_id, h, seat
                );
                stats.hands_aborted += 1;
                return Err(());
            }
        }
    }
}

// ====== СИНХРОНИЗАЦИЯ СТЕКОВ ==============================================

fn sync_and_eliminate(
    lobby: &mut TournamentLobby,
    tid: TournamentId,
    mgr: &mut TableManager,
    table_id: TableId,
    elim_order: &mut Vec<PlayerId>,
) {
    let t = mgr.table_mut(table_id).unwrap();

    let mut seats: Vec<(usize, PlayerId, Chips)> = vec![];
    for (i, s) in t.seats.iter().enumerate() {
        if let Some(p) = s {
            seats.push((i, p.player_id, p.stack));
        }
    }

    let tour = lobby.get_mut(tid).unwrap();

    for (i, pid, st) in seats {
        if st.0 == 0 {
            t.seats[i] = None;
            tour.unregister_player(pid);
            elim_order.push(pid);
        } else if let Some(r) = tour.registration_for_mut(pid) {
            r.stack = st;
        }
    }
}

// ====== SEATING ============================================================

#[derive(Clone)]
struct RuntimeSeat {
    player_id: PlayerId,
    seat: u8,
    stack: Chips,
    entries_used: u32,
}

#[derive(Clone)]
struct RuntimeTable {
    table_id: u32,
    players: Vec<RuntimeSeat>,
}

fn seating(tour: &Tournament) -> Vec<RuntimeTable> {
    let ts = tour.config.table_size as usize;
    let mut players: Vec<PlayerId> = tour.players().collect();
    players.sort_unstable();

    let mut out = vec![];
    let mut tid = 1;

    for chunk in players.chunks(ts) {
        let mut v = vec![];
        for (i, &pid) in chunk.iter().enumerate() {
            let r = tour.registration_for(pid).unwrap();
            v.push(RuntimeSeat {
                player_id: pid,
                seat: i as u8,
                stack: r.stack,
                entries_used: r.entries_used,
            });
        }
        out.push(RuntimeTable {
            table_id: tid,
            players: v,
        });
        tid += 1;
    }
    out
}

// ====== BLIND LEVELS =======================================================

#[derive(Clone)]
struct BlindLevel {
    lvl: u32,
    sb: Chips,
    bb: Chips,
    ante: Chips,
}

fn levels() -> Vec<BlindLevel> {
    vec![
        (50, 100),
        (75, 150),
        (100, 200),
        (150, 300),
        (200, 400),
        (300, 600),
        (400, 800),
        (600, 1200),
        (800, 1600),
        (1000, 2000),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (sb, bb))| BlindLevel {
        lvl: (i + 1) as u32,
        sb: Chips::new(sb),
        bb: Chips::new(bb),
        ante: Chips::new(0),
    })
    .collect()
}

// ====== ОДИН ПОЛНЫЙ УРОВЕНЬ ТУРНИРА =======================================

fn run_level(
    lobby: &mut TournamentLobby,
    tid: TournamentId,
    lvl: &BlindLevel,
    idg: &mut IdGenerator,
    rng: &mut SystemRng,
    elim: &mut Vec<PlayerId>,
    stats: &mut RuntimeStats,
    time_ctrl: &mut TimeController,
) -> usize {
    let count = lobby.get(tid).unwrap().current_player_count();
    if count <= 1 {
        return count;
    }

    println!(
        "\n=== LEVEL {}  {} / {} (players={}) ===",
        lvl.lvl, lvl.sb.0, lvl.bb.0, count
    );

    let rt = {
        let t = lobby.get(tid).unwrap();
        seating(t)
    };

    let mut mgr = TableManager::new();
    let mut profiles = vec![];

    for (i, tbl) in rt.iter().enumerate() {
        let real_tid = idg.next_table_id();

        let stakes = TableStakes::new(
            lvl.sb,
            lvl.bb,
            if lvl.ante.0 > 0 {
                AnteType::Classic
            } else {
                AnteType::None
            },
            lvl.ante,
        );

        let cfg = TableConfig {
            max_seats: lobby.get(tid).unwrap().config.table_size,
            table_type: TableType::Tournament,
            stakes,
            allow_straddle: false,
            allow_run_it_twice: false,
        };

        let ms = cfg.max_seats as usize;

        let mut table = Table::new(
            real_tid,
            format!("T{}-L{}-{}", tid, lvl.lvl, i + 1),
            cfg,
        );

        if table.seats.len() < ms {
            table.seats.resize(ms, None);
        }

        for p in &tbl.players {
            table.seats[p.seat as usize] = Some(PlayerAtTable::new(p.player_id, p.stack));
        }

        mgr.add_table(table);

        let prof = match i % 4 {
            0 => TableProfile::TightPassive,
            1 => TableProfile::LooseAggressive,
            2 => TableProfile::PushOrFold,
            _ => TableProfile::Mixed,
        };
        profiles.push((real_tid, prof));
    }

    for h in 0..HANDS_PER_LEVEL {
        for (tid2, prof) in &profiles {
            let alive = mgr
                .table(*tid2)
                .unwrap()
                .seats
                .iter()
                .filter(|s| s.is_some())
                .count();

            if alive < 2 {
                continue;
            }

            let hid = idg.next_hand_id();
            if mgr.start_hand(*tid2, rng, hid).is_err() {
                continue;
            }
            stats.hands_planned += 1;

            let r = play_hand(&mut mgr, *tid2, *prof, h, stats, time_ctrl);
            if let Ok(_) = r {
                sync_and_eliminate(lobby, tid, &mut mgr, *tid2, elim);
            }
        }

        if lobby.get(tid).unwrap().current_player_count() <= 1 {
            break;
        }
    }

    let left = lobby.get(tid).unwrap().current_player_count();
    println!("Players left after level {} = {}", lvl.lvl, left);
    left
}

// ====== TOURNAMENT SIMULATION ==============================================

fn simulate(lobby: &mut TournamentLobby, tid: TournamentId) {
    let mut idg = IdGenerator::new();
    let mut rng = SystemRng::default();
    let mut elim = vec![];
    let mut stats = RuntimeStats::default();

    // Контроллер времени турнира: 20 сек на ход, 60 сек банка по 10 сек.
    let mut time_ctrl = TimeController::new(TimeRules::standard());
    {
        let t = lobby.get(tid).unwrap();
        time_ctrl.init_players(t.players());
    }

    {
        let t = lobby.get_mut(tid).unwrap();
        t.status = TournamentStatus::Running;
    }

    let blind_levels = levels();
    let mut last_lvl: Option<BlindLevel> = None;

    // 1) Играем все заданные уровни
    for lvl in &blind_levels {
        last_lvl = Some(lvl.clone());

        let left = run_level(
            lobby,
            tid,
            lvl,
            &mut idg,
            &mut rng,
            &mut elim,
            &mut stats,
            &mut time_ctrl,
        );

        if left <= 1 {
            break;
        }
    }

    // 2) Если игроков всё ещё > 1 — крутим последний уровень до конца
    if lobby.get(tid).unwrap().current_player_count() > 1 {
        let last_lvl = last_lvl.expect("there must be at least one blind level defined");

        loop {
            let left = run_level(
                lobby,
                tid,
                &last_lvl,
                &mut idg,
                &mut rng,
                &mut elim,
                &mut stats,
                &mut time_ctrl,
            );

            if left <= 1 {
                break;
            }
        }
    }

    // 3) Финальный статус
    {
        let t = lobby.get_mut(tid).unwrap();
        if t.current_player_count() <= 1 {
            t.status = TournamentStatus::Finished;
        }
    }

    // 4) Финалка
    let t = lobby.get(tid).unwrap();

    println!("\n=== FINAL RESULT (tournament id={}) ===", tid);
    println!("Status: {:?}", t.status);
    println!("Players left: {}", t.current_player_count());

    println!("Elimination order:");
    for (i, pid) in elim.iter().enumerate() {
        println!("  bust #{} -> player {}", i + 1, pid);
    }

    if t.current_player_count() == 1 {
        let w = t.players().next().unwrap();
        println!("WINNER: player {}", w);
    }

    println!(
        "[STATS] planned={} finished={} no_actor={} aborted={}",
        stats.hands_planned,
        stats.hands_finished,
        stats.hands_finished_no_actor,
        stats.hands_aborted
    );
}

// ====== MAIN ==============================================================

fn main() {
    let mut lobby = TournamentLobby::new();

    let cfg = TournamentConfig {
        name: "Real MTT Simulation".to_string(),
        starting_stack: Chips::new(10000),
        table_size: 9,
        freezeout: false,
        reentry_allowed: true,
        max_players: 2000,
        max_reentries_per_player: 3,
    };

    let tid = lobby.create_tournament(cfg);

    // Регистрируем 45 игроков
    for pid in 1..=45 {
        let _ = lobby.register_player(tid, pid);
    }

    println!("Starting REAL tournament simulation...\n");
    simulate(&mut lobby, tid);
}
