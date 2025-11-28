// tests/engine_integration_tests.rs
//
// E. Integration Tests (10 тестов)
//
// Здесь мы тестируем ИНТЕГРАЦИЮ домена турнира + RNG:
//
//  1) 9-max: регистрации, старт, рассадка по одному столу.
//  2) 9-max: последовательные bust → один победитель с корректными places.
//  3) Турнир с 20 игроками: seat_players_evenly даёт разумное распределение по столам.
//  4) Турнир с 20 игроками: искусственно разбалансируем столы → compute_rebalance_moves выравнивает.
//  5) DeterministicRng: одинаковый seed даёт одинаковый shuffle.
//  6) DeterministicRng: разные seed дают разный shuffle.
//  7) Полный турнир (только bust-логика) под управлением RNG: с одинаковым seed порядок bust детерминирован.
//  8) То же, но с разными seed: порядок bust отличается.
//  9) Edge-case: «все all-in префлоп» моделируем как один "раунд" bust всех, кроме победителя.
// 10) Edge-case: те же "all-in", но с разными стеками — finishing_place и winner корректны даже при перекошенных стеках.

use poker_engine::domain::{PlayerId, TournamentId};
use poker_engine::domain::chips::Chips;
use poker_engine::domain::blinds::{AnteType, BlindLevel, BlindStructure};
use poker_engine::domain::tournament::{
    TableBalancingConfig, Tournament, TournamentConfig, TournamentError, TournamentScheduleConfig,
    TournamentStatus,
};
use poker_engine::engine::RandomSource;
use poker_engine::infra::rng::DeterministicRng;

// ---------------------------------------------------
// ВСПОМОГАТЕЛЬНЫЕ КОНСТРУКТОРЫ ДЛЯ КОНФИГОВ ТУРНИРА
// ---------------------------------------------------

fn integration_blind_structure() -> BlindStructure {
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
            BlindLevel {
                level: 3,
                small_blind: Chips(200),
                big_blind: Chips(400),
                ante: Chips(0),
                ante_type: AnteType::None,
                duration_minutes: 10,
            },
        ],
    }
}

fn integration_schedule() -> TournamentScheduleConfig {
    TournamentScheduleConfig {
        scheduled_start_ts: 0,
        allow_start_earlier: true,
        break_every_minutes: 60,
        break_duration_minutes: 5,
    }
}

fn integration_balancing() -> TableBalancingConfig {
    TableBalancingConfig {
        enabled: true,
        max_seat_diff: 1,
    }
}

fn make_tournament_config(
    name: &str,
    max_players: u32,
    min_players_to_start: u32,
    table_size: u8,
) -> TournamentConfig {
    TournamentConfig {
        name: name.to_string(),
        description: None,
        starting_stack: Chips(10_000),
        max_players,
        min_players_to_start,
        table_size,
        freezeout: true,
        reentry_allowed: false,
        max_entries_per_player: 1,
        late_reg_level: 0,
        blind_structure: integration_blind_structure(),
        auto_approve: true,
        schedule: integration_schedule(),
        balancing: integration_balancing(),
    }
}

/// Создаёт турнир и регистрирует `num_players` игроков (id = 1..=num_players).
fn create_tournament_with_players(
    id: TournamentId,
    owner: PlayerId,
    num_players: usize,
    table_size: u8,
) -> Tournament {
    let cfg = make_tournament_config(
        "IntegrationTournament",
        num_players as u32,
        2,
        table_size,
    );
    let mut t = Tournament::new(id, owner, cfg).expect("Tournament::new must succeed");

    for pid in 1..=num_players {
        let pid: PlayerId = pid as PlayerId;
        t.register_player(pid)
            .unwrap_or_else(|e| panic!("register_player({pid}) failed: {e:?}"));
    }

    t
}

// -----------------------------------------
// 1) 9-max: рассадка по одному столу
// -----------------------------------------

#[test]
fn nine_max_seating_single_table_has_9_players_sorted() {
    let owner: PlayerId = 999;
    let mut t = create_tournament_with_players(1, owner, 9, 9);

    // Переводим в Running, имитируя реальный старт.
    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    // 9-max: один стол, table_id начинаем с 100.
    let seating = t.seat_players_evenly(9, 100);

    assert_eq!(
        seating.len(),
        1,
        "При 9 игроках и table_size=9 должен быть один стол"
    );
    let (table_id, seated) = &seating[0];
    assert_eq!(*table_id, 100, "Ожидаем table_id = 100");
    assert_eq!(
        seated.len(),
        9,
        "На единственном 9-max столе должно быть 9 игроков"
    );

    let mut sorted = seated.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        (1u64..=9u64).collect::<Vec<_>>(),
        "Игроки должны быть отсортированы по id от 1 до 9"
    );

    // Проверяем, что в регистрациях у всех игроков стоит table_id и seat_index.
    for reg in t.active_players() {
        assert_eq!(
            reg.table_id,
            Some(*table_id),
            "У каждого активного игрока должен быть table_id = 100"
        );
        assert!(
            reg.seat_index.is_some(),
            "seat_index для посаженных игроков должен быть Some(..)"
        );
    }
}

// ------------------------------------------------------
// 2) 9-max: bust до одного победителя, корректные places
// ------------------------------------------------------

#[test]
fn nine_max_tournament_busts_down_to_single_winner_with_correct_places() {
    let owner: PlayerId = 999;
    let mut t = create_tournament_with_players(2, owner, 9, 9);

    // Стартуем турнир "по-настоящему" через start()
    let now_ts: u64 = 1_000_000;
    t.start(now_ts)
        .expect("start() must succeed for 9 registered players");

    assert_eq!(
        t.status,
        TournamentStatus::Running,
        "После start турнир должен быть в статусе Running"
    );
    assert_eq!(
        t.total_entries, 9,
        "total_entries при старте должен быть равен 9"
    );

    // Bust-им 8 игроков (1..=8), оставляем 9-го победителем.
    let mut places = Vec::new();
    for pid in 1u64..=8u64 {
        let place = t
            .mark_player_busted(pid)
            .unwrap_or_else(|e| panic!("mark_player_busted({pid}) failed: {e:?}"));
        places.push(place);
        // Пока ещё больше одного активного игрока, турнир не обязан быть Finished.
        if pid < 8 {
            assert!(
                !t.is_finished(),
                "До последнего bust из 8 турнир не должен завершаться"
            );
        }
    }

    // После восьмого bust из 9 участников должен остаться 1 активный,
    // и турнир обязан завершиться.
    assert!(t.is_finished(), "После 8 bust из 9 турнир должен быть Finished");
    assert_eq!(
        t.active_player_count(),
        1,
        "Должен остаться ровно один активный игрок"
    );

    // Проверим, что места убывают от 9 до 2 (по текущей логике mark_player_busted).
    // Первый вылетевший при total_entries=9 получает place=9,
    // затем 8, 7, ..., 2.
    assert_eq!(
        places,
        vec![9, 8, 7, 6, 5, 4, 3, 2],
        "Порядок finishing_place должен быть [9,8,7,6,5,4,3,2]"
    );

    let winner_id = t.winner_id.expect("Winner must be set");
    assert_eq!(
        winner_id, 9,
        "Ожидаем, что последний не выбитый игрок (id=9) станет победителем"
    );

    let reg_winner = t
        .registrations
        .get(&winner_id)
        .expect("Winner must be in registrations");
    assert_eq!(
        reg_winner.finishing_place,
        Some(1),
        "Победителю должно быть присвоено место 1"
    );
}

// ---------------------------------------------------------------------
// 3) Турнир с 20 игроками: seat_players_evenly создаёт разумное распределение
// ---------------------------------------------------------------------

#[test]
fn tournament_20_players_balancing_creates_multiple_tables() {
    let owner: PlayerId = 1000;
    let mut t = create_tournament_with_players(3, owner, 20, 9);

    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    let seating = t.seat_players_evenly(9, 1);

    assert!(
        seating.len() >= 2,
        "При 20 игроках и table_size=9 должно быть как минимум два стола"
    );

    // Соберём количество игроков по каждому столу.
    let mut counts: Vec<usize> = seating.iter().map(|(_, players)| players.len()).collect();
    counts.sort_unstable();

    let min_count = *counts.first().unwrap();
    let max_count = *counts.last().unwrap();

    assert!(
        max_count - min_count <= t.config.balancing.max_seat_diff as usize,
        "seat_players_evenly должен гарантировать разницу по столам не больше max_seat_diff"
    );

    // Проверим, что все 20 игроков участвуют и нет дублей
    let mut all_players = Vec::new();
    for (_, players) in seating {
        all_players.extend(players);
    }
    all_players.sort_unstable();
    all_players.dedup();
    assert_eq!(
        all_players.len(),
        20,
        "Все 20 игроков должны быть посажены без дублей"
    );
}

// ---------------------------------------------------------------------
// 4) Турнир с 20 игроками: искусственный дисбаланс → ребаланс выравнивает
// ---------------------------------------------------------------------

#[test]
fn tournament_20_players_rebalance_moves_players_to_reduce_imbalance() {
    let owner: PlayerId = 1001;
    let mut t = create_tournament_with_players(4, owner, 20, 9);

    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    // Искусственно делаем сильный дисбаланс:
    // первые 18 игроков сидят за столом 1, последние 2 — за столом 2.
    let mut regs: Vec<_> = t.registrations.values_mut().collect();
    regs.sort_by_key(|r| r.player_id);

    for (idx, reg) in regs.iter_mut().enumerate() {
        if idx < 18 {
            reg.table_id = Some(1);
        } else {
            reg.table_id = Some(2);
        }
        reg.seat_index = None;
    }

    // compute_rebalance_moves должен сгенерировать перестановки.
    let moves = t.compute_rebalance_moves();
    assert!(
        !moves.is_empty(),
        "При дисбалансе 18 vs 2 compute_rebalance_moves должен вернуть хотя бы одну перестановку"
    );

    t.apply_rebalance_moves(&moves);

    // После применения перестановок считаем распределение ещё раз.
    use std::collections::HashMap;
    let mut table_counts: HashMap<u64, usize> = HashMap::new();
    for reg in t.active_players() {
        if let Some(tid) = reg.table_id {
            *table_counts.entry(tid).or_default() += 1;
        }
    }

    assert!(
        table_counts.len() >= 2,
        "После ребаланса по-прежнему должно быть как минимум два стола"
    );

    let mut counts: Vec<usize> = table_counts.values().copied().collect();
    counts.sort_unstable();
    let min_count = *counts.first().unwrap();
    let max_count = *counts.last().unwrap();

    assert!(
        max_count - min_count <= t.config.balancing.max_seat_diff as usize,
        "После apply_rebalance_moves разница по количеству игроков должна быть не больше max_seat_diff"
    );
}

// --------------------------------------------------------
// 5) RNG: одинаковый seed → одинаковый shuffle колоды
// --------------------------------------------------------

#[test]
fn deterministic_rng_same_seed_produces_same_shuffle() {
    let mut deck1: Vec<u32> = (0..52).collect();
    let mut deck2: Vec<u32> = (0..52).collect();

    let mut rng1 = DeterministicRng::from_u64(42);
    let mut rng2 = DeterministicRng::from_u64(42);

    rng1.shuffle(&mut deck1);
    rng2.shuffle(&mut deck2);

    assert_eq!(
        deck1, deck2,
        "DeterministicRng с одинаковым seed должен давать идентичный shuffle"
    );
}

// --------------------------------------------------------
// 6) RNG: разные seed → разный shuffle колоды
// --------------------------------------------------------

#[test]
fn deterministic_rng_different_seeds_produce_different_shuffle() {
    let mut deck1: Vec<u32> = (0..52).collect();
    let mut deck2: Vec<u32> = (0..52).collect();

    let mut rng1 = DeterministicRng::from_u64(42);
    let mut rng2 = DeterministicRng::from_u64(43);

    rng1.shuffle(&mut deck1);
    rng2.shuffle(&mut deck2);

    assert_ne!(
        deck1, deck2,
        "DeterministicRng с разными seed должен давать различный shuffle"
    );
}

// --------------------------------------------------------
// Вспомогательный лог для "полного турнира"
// --------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
struct BustEvent {
    player_id: PlayerId,
    place: u32,
}

fn run_seed_driven_tournament(seed: u64, num_players: usize) -> (Tournament, Vec<BustEvent>) {
    let owner: PlayerId = 5000;
    let mut t = create_tournament_with_players(10_000 + seed as u64, owner, num_players, 9);

    // Запускаем турнир штатно.
    let now_ts: u64 = 1_000_000;
    t.start(now_ts)
        .expect("start() must succeed in seed-driven tournament");

    let mut rng = DeterministicRng::from_u64(seed);
    let mut events = Vec::new();

    // Пока не остался один победитель.
    while !t.is_finished() {
        // Собираем список активных игроков.
        let mut active: Vec<PlayerId> = t.active_players().map(|r| r.player_id).collect();
        active.sort_unstable();

        if active.len() <= 1 {
            // check_and_finish_if_needed внутри mark_player_busted сам завершит турнир.
            break;
        }

        // Перемешиваем порядок активных игроков и выбиваем первого.
        rng.shuffle(&mut active);
        let target = active[0];

        let place = t
            .mark_player_busted(target)
            .unwrap_or_else(|e| panic!("mark_player_busted({target}) failed: {e:?}"));

        events.push(BustEvent { player_id: target, place });
    }

    (t, events)
}

// --------------------------------------------------------------------
// 7) Полный турнир под RNG: одинаковый seed → одинаковый лог bust-ов
// --------------------------------------------------------------------

#[test]
fn full_tournament_replay_with_same_seed_produces_identical_bust_log() {
    let (t1, log1) = run_seed_driven_tournament(123, 12);
    let (t2, log2) = run_seed_driven_tournament(123, 12);

    assert_eq!(
        log1, log2,
        "При одинаковом seed лог bust-ов должен быть детерминированным"
    );
    assert_eq!(
        t1.total_entries, t2.total_entries,
        "total_entries должен совпадать при реплее"
    );
    assert_eq!(
        t1.winner_id, t2.winner_id,
        "Победитель турнира при одинаковом seed должен быть тот же"
    );
}

// --------------------------------------------------------------------
// 8) Полный турнир под RNG: разные seed → разный порядок bust-ов
// --------------------------------------------------------------------

#[test]
fn full_tournament_replay_with_different_seeds_produces_different_bust_log() {
    let (_t1, log1) = run_seed_driven_tournament(321, 12);
    let (_t2, log2) = run_seed_driven_tournament(654, 12);

    assert_eq!(
        log1.len(),
        log2.len(),
        "Число bust-ов при одинаковом количестве игроков должно совпадать"
    );

    assert_ne!(
        log1, log2,
        "При разных seed порядок bust-ов должен отличаться"
    );
}

// ---------------------------------------------------------------------
// 9) Edge-case: "все all-in префлоп" → один раунд bust всех, кроме победителя
// ---------------------------------------------------------------------

#[test]
fn all_players_all_in_preflop_simulated_as_single_round_produces_single_winner() {
    let owner: PlayerId = 7000;
    let mut t = create_tournament_with_players(7_001, owner, 4, 9);

    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    let mut rng = DeterministicRng::from_u64(777);

    // Все "all-in": с т.з. турнира считаем, что все рискуют стеком,
    // и в итоге один становится победителем, остальные выбывают
    // в рамках "одной" логической раздачи.
    let mut active: Vec<PlayerId> = t.active_players().map(|r| r.player_id).collect();
    active.sort_unstable();

    rng.shuffle(&mut active);
    let winner_id = active[0];
    let losers = &active[1..];

    let mut places = Vec::new();
    for &pid in losers {
        let place = t
            .mark_player_busted(pid)
            .unwrap_or_else(|e| panic!("bust({pid}) failed in all-in edge case: {e:?}"));
        places.push(place);
    }

    // После bust всех лузеров должен остаться один активный игрок,
    // турнир должен завершиться.
    assert!(t.is_finished(), "Турнир должен быть Finished");
    assert_eq!(
        t.active_player_count(),
        1,
        "После bust всех остальных должен остаться один активный игрок"
    );

    assert_eq!(
        t.winner_id,
        Some(winner_id),
        "Победитель edge-case all-in должен совпадать с выбранным winner_id"
    );

    // Проверим, что всем лузерам проставлены места > 1 и нет дубликатов.
    let mut sorted_places = places.clone();
    sorted_places.sort_unstable();
    sorted_places.dedup();
    assert_eq!(
        sorted_places.len(),
        places.len(),
        "finishing_place для лузеров должны быть уникальными"
    );
    assert!(
        sorted_places.iter().all(|&p| p >= 2),
        "finishing_place для лузеров должны быть >= 2"
    );
}

// ---------------------------------------------------------------------
// 10) Edge-case: те же all-in, но с разными стеками — логика мест стабильна
// ---------------------------------------------------------------------

#[test]
fn all_players_all_in_with_uneven_stacks_still_produces_consistent_ranking() {
    let owner: PlayerId = 8000;
    let mut t = create_tournament_with_players(8_001, owner, 4, 9);

    // Настроим перекошенные стеки.
    // Важно: mark_player_busted не опирается напрямую на total_chips,
    // но здесь мы моделируем сценарий, когда стеки сильно различаются.
    let mut regs: Vec<_> = t.registrations.values_mut().collect();
    regs.sort_by_key(|r| r.player_id);

    regs[0].total_chips = Chips(1_000);
    regs[1].total_chips = Chips(2_000);
    regs[2].total_chips = Chips(4_000);
    regs[3].total_chips = Chips(10_000);

    t.status = TournamentStatus::Running;
    t.total_entries = t.active_player_count() as u32;

    let mut rng = DeterministicRng::from_u64(1_234);

    let mut active: Vec<PlayerId> = t.active_players().map(|r| r.player_id).collect();
    active.sort_unstable();

    rng.shuffle(&mut active);
    let winner_id = active[0];
    let losers = &active[1..];

    let mut places = Vec::new();
    for &pid in losers {
        let place = t
            .mark_player_busted(pid)
            .unwrap_or_else(|e| panic!("bust({pid}) failed in uneven-stack all-in: {e:?}"));
        places.push(place);
    }

    assert!(
        t.is_finished(),
        "После bust всех, кроме одного, турнир должен быть Finished"
    );
    assert_eq!(
        t.winner_id,
        Some(winner_id),
        "Победителем должен быть игрок, которого мы оставили не выбитым"
    );

    // Проверяем, что finishing_place распределились корректно и уникально.
    let mut sorted_places = places.clone();
    sorted_places.sort_unstable();
    sorted_places.dedup();
    assert_eq!(
        sorted_places.len(),
        places.len(),
        "finishing_place для лузеров должны быть уникальными"
    );

    // Победитель должен иметь place=1.
    let winner_reg = t
        .registrations
        .get(&winner_id)
        .expect("winner reg must exist");
    assert_eq!(
        winner_reg.finishing_place,
        Some(1),
        "Победитель должен иметь finishing_place = 1"
    );
}
