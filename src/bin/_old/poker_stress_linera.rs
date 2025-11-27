// src/bin/poker_stress_linera.rs
//
// Стресс-тест: много столов, много рук.
// Этап пути B: включены all-in, агрессивное поведение, проверяем сайд-поты.

use poker_engine::domain::blinds::AnteType;
use poker_engine::domain::chips::Chips;
use poker_engine::domain::hand::Street;
use poker_engine::domain::player::PlayerAtTable;
use poker_engine::domain::table::{Table, TableConfig, TableStakes, TableType};
use poker_engine::domain::{HandId, PlayerId, SeatIndex, TableId};
use poker_engine::engine::{HandStatus, PlayerAction, PlayerActionKind, TableManager};
use poker_engine::infra::{IdGenerator, SystemRng};

/// Сколько всего игроков хотим посадить в стресс-тесте.
const TOTAL_PLAYERS: usize = 1000;

/// Сколько мест за столом в среднем (6-макс).
const SEATS_PER_TABLE: usize = 6;

/// Сколько рук играем на каждом столе.
const HANDS_PER_TABLE: u32 = 50;

/// Максимальное количество шагов в одной раздаче (подстраховка от бесконечного цикла).
const MAX_STEPS_PER_HAND: u32 = 200;

/// Профиль поведения стола.
#[derive(Copy, Clone, Debug)]
enum TableProfile {
    TightPassive,    // максимально аккуратно: check/call, редкие маленькие ставки
    LooseAggressive, // чаще bet/raise и иногда all-in
    PushOrFold,      // логика "push or fold": больше фолдов и пушей
    Mixed,           // смесь поведения
}

/// Простая статистика по стресс-тесту.
#[derive(Default)]
struct StressStats {
    tables: u32,
    /// Сколько рук реально удалось стартовать (успешный start_hand).
    hands_planned: u64,
    /// Сколько рук успешно доиграно до конца (собрали summary / завершили корректно).
    hands_finished: u64,
    /// Сколько рук оборвано из-за бага/аномалии/лимита шагов.
    hands_aborted: u64,
    /// Ошибки при start_hand (Engine/Manager error).
    start_errors: u64,
    /// Ошибки при apply_action (Engine/Manager error).
    apply_errors: u64,
    /// Суммарный пот по всем доигранным рукам.
    total_pot: u64,
    /// Максимальный пот среди всех доигранных рук.
    max_pot: u64,
    /// Сколько рук дошло до шоудауна.
    showdowns: u64,
}

/// Результат одной руки.
enum HandResult {
    /// Обычное завершение руки (через HandStatus::Finished или "чистое" окончание).
    FinishedNormal,
    /// Крайний кейс: HandEngine ещё есть, но актёра нет.
    /// Мы считаем эту руку завершённой, но новые руки за этим столом не стартуем.
    FinishedNoActorEngineAlive,
}

fn main() {
    println!("poker_stress_test: стартуем стресс-тест покерного движка (нормальная логика с all-in)…");

    let mut id_gen = IdGenerator::new();
    let mut rng = SystemRng::default();
    let mut manager = TableManager::new();
    let mut stats = StressStats::default();

    // Конфиг кэш-стола 50/100 без анте.
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

    // 1. Создаём столы и рассаживаем 1000 игроков.
    let table_descriptors = build_tables_with_profiles(
        &mut manager,
        &mut id_gen,
        &config,
    );

    println!(
        "[STRESS] Создано {} столов, всего игроков ~{}.",
        table_descriptors.len(),
        TOTAL_PLAYERS,
    );

    // 2. Прогоняем для каждого стола по HANDS_PER_TABLE рук.
    for (table_index, (table_id, profile)) in table_descriptors.iter().enumerate() {
        stats.tables += 1;
        println!(
            "[STRESS] Стол #{} (id={}) с профилем {:?}: начинаем {} рук.",
            table_index + 1,
            table_id,
            profile,
            HANDS_PER_TABLE
        );

        let mut hands_for_this_table: u64 = 0;
        let mut stop_for_this_table = false;

        for hand_seq in 0..HANDS_PER_TABLE {
            if stop_for_this_table {
                break;
            }

            let hand_id: HandId = id_gen.next_hand_id();

            // Пытаемся стартовать новую руку.
            match manager.start_hand(*table_id, &mut rng, hand_id) {
                Ok(()) => {
                    stats.hands_planned += 1;
                    hands_for_this_table += 1;

                    match play_one_hand(
                        &mut manager,
                        *table_id,
                        *profile,
                        hand_seq,
                        &mut stats,
                    ) {
                        Ok(HandResult::FinishedNormal) => {
                            // Всё ок, рука доиграна / завершена ожидаемо.
                        }
                        Ok(HandResult::FinishedNoActorEngineAlive) => {
                            // Крайний кейс: движок ещё живёт, но актёра нет.
                            // Логируем и прекращаем стресс за этим столом,
                            // чтобы не долбить такие же "мгновенные" руки.
                            println!(
                                "[STRESS][table_id={}] Рука #{} завершена без актёра (HandEngine ещё активен). Новые руки за этим столом не стартуем.",
                                table_id,
                                hand_seq + 1
                            );
                            stop_for_this_table = true;
                        }
                        Err(()) => {
                            stats.hands_aborted += 1;
                            println!(
                                "[STRESS][table_id={}] Рука #{} оборвана из-за ошибки/лимита шагов/аномалии. Прекращаем игру за этим столом.",
                                table_id,
                                hand_seq + 1
                            );
                            stop_for_this_table = true;
                        }
                    }
                }
                Err(e) => {
                    stats.start_errors += 1;
                    println!(
                        "[STRESS][table_id={}] ОШИБКА в start_hand (hand_seq={}): {:?}",
                        table_id,
                        hand_seq,
                        e
                    );
                    // Если не можем даже стартовать руку — дальше за этим столом идти смысла нет.
                    break;
                }
            }
        }

        println!(
            "[STRESS][table_id={}] завершили попытку сыграть {} рук.",
            table_id, hands_for_this_table
        );
    }

    // 3. Итоговый вывод.
    println!();
    println!("=========== STRESS TEST SUMMARY (NORMAL LOGIC + ALL-IN) ===========");
    println!("Столов участвовало: {}", stats.tables);
    println!("Рук запущено (start_hand OK): {}", stats.hands_planned);
    println!("Рук доиграно:                 {}", stats.hands_finished);
    println!("Рук оборвано:                 {}", stats.hands_aborted);
    println!("Ошибок start_hand:            {}", stats.start_errors);
    println!("Ошибок apply_action:          {}", stats.apply_errors);
    println!("Суммарный пот:                {}", stats.total_pot);
    println!("Максимальный пот:             {}", stats.max_pot);
    println!("Рук с шоудауном:              {}", stats.showdowns);
    println!("===================================================================");
    println!("poker_stress_test: завершено.");
}

/// Создаём столы, рассаживаем ~1000 игроков и назначаем каждому столу профиль.
fn build_tables_with_profiles(
    manager: &mut TableManager,
    id_gen: &mut IdGenerator,
    config: &TableConfig,
) -> Vec<(TableId, TableProfile)> {
    let mut result = Vec::new();
    let mut players_left = TOTAL_PLAYERS;
    let mut table_index: usize = 0;

    while players_left > 0 {
        let table_id: TableId = id_gen.next_table_id();
        let mut table = Table::new(
            table_id,
            format!("STRESS TABLE {}", table_index + 1),
            config.clone(),
        );

        // Сколько игроков посадим за этот стол.
        let seats_here = if players_left >= SEATS_PER_TABLE {
            SEATS_PER_TABLE
        } else {
            players_left
        };

        for seat in 0..seats_here {
            let pid: PlayerId = id_gen.next_player_id();
            table.seats[seat] = Some(PlayerAtTable::new(pid, Chips::new(10_000)));
        }

        let profile = match table_index % 4 {
            0 => TableProfile::TightPassive,
            1 => TableProfile::LooseAggressive,
            2 => TableProfile::PushOrFold,
            _ => TableProfile::Mixed,
        };

        manager.add_table(table);
        result.push((table_id, profile));

        players_left -= seats_here;
        table_index += 1;
    }

    result
}

/// Одна рука на одном столе.
///
/// ЛОГИКА:
/// - Есть связка `HandEngine` + `current_actor_seat`.
/// - Если нет HandEngine и нет актёра → рука точно завершена.
/// - Если HandEngine есть, но актёра нет → считаем, что рука завершилась
///   мгновенно внутри движка (крайний кейс), и тоже считаем её доигранной.
/// - Баги только там, где HandEngine и current_actor противоречат друг другу
///   (например, нет HandEngine, но есть current_actor).
fn play_one_hand(
    manager: &mut TableManager,
    table_id: TableId,
    profile: TableProfile,
    hand_seq: u32,
    stats: &mut StressStats,
) -> Result<HandResult, ()> {
    use poker_engine::engine::ManagerError;

    let mut step: u32 = 0;

    loop {
        step += 1;
        if step > MAX_STEPS_PER_HAND {
            println!(
                "[STRESS][table_id={}] hand_seq={} превышен лимит шагов ({}) — вероятный баг логики.",
                table_id, hand_seq, MAX_STEPS_PER_HAND
            );
            return Err(());
        }

        // Сначала проверяем наличие HandEngine и текущего актёра.
        let engine_exists = manager.hand_engine(table_id).is_some();
        let seat_opt = manager.current_actor_seat(table_id);

        match (engine_exists, seat_opt) {
            // Нет HandEngine и нет текущего актёра — рука завершена (нормальный кейс).
            (false, None) => {
                stats.hands_finished += 1;
                return Ok(HandResult::FinishedNormal);
            }
            // Есть HandEngine и есть актёр — нормальная работа.
            (true, Some(seat)) => {
                let table_ref = match manager.table(table_id) {
                    Some(t) => t,
                    None => {
                        println!(
                            "[STRESS][table_id={}] hand_seq={} BUG: стол исчез из менеджера.",
                            table_id, hand_seq
                        );
                        return Err(());
                    }
                };

                let engine_ref = match manager.hand_engine(table_id) {
                    Some(e) => e,
                    None => {
                        println!(
                            "[STRESS][table_id={}] hand_seq={} BUG: есть current_actor, но нет HandEngine (гонка состояний).",
                            table_id, hand_seq
                        );
                        return Err(());
                    }
                };

                let seat_idx = seat as usize;
                let player = match table_ref.seats.get(seat_idx).and_then(|s| s.as_ref()) {
                    Some(p) => p,
                    None => {
                        println!(
                            "[STRESS][table_id={}] hand_seq={} BUG: current_actor указывает на пустое место seat={}.",
                            table_id, hand_seq, seat
                        );
                        return Err(());
                    }
                };

                let action_kind = pick_action_for_profile(
                    profile,
                    hand_seq,
                    step,
                    table_ref,
                    engine_ref,
                    seat,
                    player,
                );

                let action = PlayerAction {
                    player_id: player.player_id,
                    seat,
                    kind: action_kind,
                };

                match manager.apply_action(table_id, action) {
                    Ok(HandStatus::Ongoing) => {
                        // продолжаем раздачу
                    }
                    Ok(HandStatus::Finished(summary, _history)) => {
                        stats.hands_finished += 1;
                        let pot = summary.total_pot.0;
                        stats.total_pot += pot;
                        if pot > stats.max_pot {
                            stats.max_pot = pot;
                        }
                        if matches!(summary.street_reached, Street::Showdown) {
                            stats.showdowns += 1;
                        }
                        return Ok(HandResult::FinishedNormal);
                    }
                    Err(ManagerError::Engine(e)) => {
                        stats.apply_errors += 1;
                        println!(
                            "[STRESS][table_id={}] hand_seq={} step={} ОШИБКА движка: {:?}",
                            table_id, hand_seq, step, e
                        );
                        return Err(());
                    }
                    Err(e) => {
                        stats.apply_errors += 1;
                        println!(
                            "[STRESS][table_id={}] hand_seq={} step={} ОШИБКА менеджера: {:?}",
                            table_id, hand_seq, step, e
                        );
                        return Err(());
                    }
                }
            }
            // Есть HandEngine, но нет current_actor — считаем, что рука
            // уже завершена мгновенно (например, авто-фолд/авто-win).
            (true, None) => {
                println!(
                    "[STRESS][table_id={}] hand_seq={} INFO: HandEngine активен, но актёра нет — считаем руку завершённой (крайний кейс).",
                    table_id, hand_seq
                );
                stats.hands_finished += 1;
                // Возвращаем специальный результат, чтобы наверху
                // можно было решить, продолжать ли стол.
                return Ok(HandResult::FinishedNoActorEngineAlive);
            }
            // Нет HandEngine, но есть current_actor — это уже реальный баг.
            (false, Some(seat)) => {
                println!(
                    "[STRESS][table_id={}] hand_seq={} BUG: нет HandEngine, но current_actor_seat = {:?}.",
                    table_id, hand_seq, seat
                );
                return Err(());
            }
        }
    }
}

/// Подсчёт суммы, которую игроку нужно доплатить до текущего бета.
fn to_call_amount(
    engine: &poker_engine::engine::HandEngine,
    player: &PlayerAtTable,
) -> u64 {
    let current_bet = engine.betting.current_bet;
    let player_bet = player.current_bet;
    if current_bet.0 > player_bet.0 {
        current_bet.0 - player_bet.0
    } else {
        0
    }
}

fn make_all_in_action(
    engine: &poker_engine::engine::HandEngine,
    player: &PlayerAtTable,
) -> PlayerActionKind {
    let current_bet = engine.betting.current_bet.0;
    let min_raise = engine.betting.min_raise.0;
    let player_total = player.current_bet.0 + player.stack.0;

    // Если даже на кол не хватает — движок сам трактует как all-in call.
    if player_total <= current_bet {
        return PlayerActionKind::Call;
    }

    // Открывающий пуш: бет на весь стек всегда легален.
    if current_bet == 0 {
        return PlayerActionKind::Bet(Chips::new(player_total));
    }

    // Есть ставка в банке, игрок хочет all-in поверх неё.
    let extra = player_total - current_bet;

    // Если "добавка" меньше минимального рейза — такой all-in
    // по сути не является легальным рейзом. Трактуем как кол.
    if min_raise > 0 && extra < min_raise {
        PlayerActionKind::Call
    } else {
        // Полноценный рейз до all-in, который удовлетворяет min_raise.
        PlayerActionKind::Raise(Chips::new(player_total))
    }
}


/// Максимально безопасная стратегия: минимум агрессии, без all-in.
fn pick_safe_action(
    table: &Table,
    engine: &poker_engine::engine::HandEngine,
    _seat: SeatIndex,
    player: &PlayerAtTable,
) -> PlayerActionKind {
    let to_call = to_call_amount(engine, player);
    let current_bet = engine.betting.current_bet.0;
    let bb = table.config.stakes.big_blind.0;
    let stack = player.stack.0;

    if to_call > 0 {
        if stack == 0 || stack < to_call {
            return PlayerActionKind::Fold;
        }
        return PlayerActionKind::Call;
    }

    match table.street {
        Street::Preflop => PlayerActionKind::Check,
        Street::Flop | Street::Turn | Street::River => {
            if current_bet > 0 {
                PlayerActionKind::Check
            } else {
                if bb > 0 && stack >= bb {
                    PlayerActionKind::Bet(Chips::new(bb))
                } else {
                    PlayerActionKind::Check
                }
            }
        }
        Street::Showdown => PlayerActionKind::Check,
    }
}

/// Профессиональная стратегия: легальные Bet/Raise + контролируемые all-in.
fn pick_professional_action(
    table: &Table,
    engine: &poker_engine::engine::HandEngine,
    seat: SeatIndex,
    player: &PlayerAtTable,
    hand_seq: u32,
    step: u32,
    profile: TableProfile,
) -> PlayerActionKind {
    let to_call = to_call_amount(engine, player);
    let current_bet = engine.betting.current_bet.0;
    let min_raise = engine.betting.min_raise.0;
    let bb = table.config.stakes.big_blind.0.max(1);
    let stack = player.stack.0;
    let player_total = player.current_bet.0 + stack;

    let pattern = (hand_seq + step + seat as u32) % 10;

    // Если стек == 0 — игрок ничего сделать не может.
    if stack == 0 {
        if to_call > 0 {
            return PlayerActionKind::Fold;
        } else {
            return PlayerActionKind::Check;
        }
    }

    // Коэффициент "агрессии" профиля.
    let aggression: u32 = match profile {
        TableProfile::TightPassive => 2,
        TableProfile::PushOrFold => 6,
        TableProfile::Mixed => 4,
        TableProfile::LooseAggressive => 7,
    };

    // ---- ЛОГИКА ALL-IN ----
    // 1) Короткий стек (≤ 8bb) → повышенный шанс пуша.
    if stack <= 8 * bb {
        let shove_chance = match profile {
            TableProfile::TightPassive => 0, // тайтовый почти никогда не пушит короткий стек.
            TableProfile::PushOrFold => 7,   // чаще всего пушит.
            TableProfile::Mixed => 4,
            TableProfile::LooseAggressive => 5,
        };

        if pattern < shove_chance {
            return make_all_in_action(engine, player);
        }
    }

    // 2) Случайные большие пуши у агро-профилей, когда уже что-то в поте.
    if player_total > 20 * bb && engine.pot.total.0 > 10 * bb {
        let crazy_shove_chance = match profile {
            TableProfile::TightPassive => 0,
            TableProfile::PushOrFold => 2,
            TableProfile::Mixed => 2,
            TableProfile::LooseAggressive => 3,
        };

        if pattern == 0 && crazy_shove_chance > 0 {
            return make_all_in_action(engine, player);
        }
    }

    // ---- ОСТАЛЬНАЯ ЛОГИКА БЕЗ ОБЯЗАТЕЛЬНОГО ALL-IN ----

    if to_call > 0 {
        if stack < to_call {
            return PlayerActionKind::Fold;
        }

        let can_raise = min_raise > 0 && (current_bet + min_raise) <= player_total;

        if can_raise && (pattern < aggression) {
            let target_total = current_bet + min_raise;
            return PlayerActionKind::Raise(Chips::new(target_total));
        } else {
            if matches!(profile, TableProfile::TightPassive | TableProfile::PushOrFold)
                && pattern == 0
            {
                return PlayerActionKind::Fold;
            }
            return PlayerActionKind::Call;
        }
    }

    // to_call == 0 (никому ничего доплачивать не нужно)
    match table.street {
        Street::Preflop => {
            if current_bet == 0 {
                if pattern < aggression {
                    let open_bet = (2 * bb).min(stack).max(bb);
                    if open_bet > 0 && open_bet <= stack {
                        return PlayerActionKind::Bet(Chips::new(open_bet));
                    } else {
                        return PlayerActionKind::Check;
                    }
                } else {
                    return PlayerActionKind::Check;
                }
            } else {
                let can_raise = min_raise > 0 && (current_bet + min_raise) <= player_total;
                if can_raise && pattern < aggression {
                    let target_total = current_bet + min_raise;
                    return PlayerActionKind::Raise(Chips::new(target_total));
                } else {
                    return PlayerActionKind::Check;
                }
            }
        }
        Street::Flop | Street::Turn | Street::River => {
            if current_bet == 0 {
                if pattern < aggression {
                    let mut desired = engine.pot.total.0 / 2;
                    if desired < bb {
                        desired = bb;
                    }
                    desired = desired.min(stack);
                    if desired >= bb && desired <= stack {
                        return PlayerActionKind::Bet(Chips::new(desired));
                    }
                }
                return PlayerActionKind::Check;
            } else {
                let can_raise = min_raise > 0 && (current_bet + min_raise) <= player_total;
                if can_raise && pattern < aggression {
                    let target_total = current_bet + min_raise;
                    return PlayerActionKind::Raise(Chips::new(target_total));
                } else {
                    return PlayerActionKind::Check;
                }
            }
        }
        Street::Showdown => PlayerActionKind::Check,
    }
}

/// Выбор действия в зависимости от профиля.
fn pick_action_for_profile(
    profile: TableProfile,
    hand_seq: u32,
    step: u32,
    table: &Table,
    engine: &poker_engine::engine::HandEngine,
    seat: SeatIndex,
    player: &PlayerAtTable,
) -> PlayerActionKind {
    let selector = (hand_seq + step + seat as u32) % 8;

    // Чем меньше порог — тем чаще используем агрессивную стратегию.
    let use_safe_threshold: u32 = match profile {
        TableProfile::TightPassive => 4,
        TableProfile::PushOrFold => 3,
        TableProfile::Mixed => 2,
        TableProfile::LooseAggressive => 1,
    };

    if selector < use_safe_threshold {
        // Осторожная линия: почти без агрессии, без all-in.
        pick_safe_action(table, engine, seat, player)
    } else {
        // Профессиональная линия: Bet/Raise + all-in по условиям.
        pick_professional_action(table, engine, seat, player, hand_seq, step, profile)
    }
}
