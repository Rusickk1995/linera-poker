// tests/engine_stress_tests.rs
//
// F. Стресс-тесты: проверяем устойчивость турнирной модели под высокой нагрузкой.
//
// 1) simulate_1000_steps_tournament_state_remains_consistent
//    - Один турнир (~50 игроков), 1000 шагов: time_tick, bust, rebalance.
//    - Проверяем, что инварианты турнира не ломаются.
//
// 2) simulate_50_full_tournaments_finish_correctly
//    - 50 турниров по 40 игроков.
//    - Каждый гоняем до завершения (или до лимита шагов), с bust + rebalance.
//    - Все турниры должны корректно завершиться.
//
// 3) random_actions_generator_keeps_tournament_consistent
//    - Один турнир, случайные операции регистрации / bust / time_tick / rebalance / noop.
//    - После каждого шага проверяем инварианты.
//
// 4) large_tournament_1000_players_many_steps_stays_consistent  (#[ignore])
//    - Большой турнир: 1000 игроков, много столов.
//    - До 20_000 шагов mix: time_tick, bust, rebalance.
//    - Проверяем, что модель не разваливается на большом N.
//
// 5) many_parallel_large_tournaments_finish_correctly          (#[ignore])
//    - Несколько параллельных турниров по 300 игроков.
//    - Каждый до завершения (или лимита шагов).
//    - Проверяем массовое завершение без нарушения инвариантов.
//

use poker_engine::domain::{PlayerId, TournamentId};
use poker_engine::domain::chips::Chips;
use poker_engine::domain::blinds::{AnteType, BlindLevel, BlindStructure};
use poker_engine::domain::tournament::{
    Tournament,
    TournamentConfig,
    TournamentStatus,
    TournamentScheduleConfig,
    TableBalancingConfig,
};
use poker_engine::infra::rng::DeterministicRng;
use poker_engine::engine::RandomSource;

// ---------------------------------------------------------
// ВСПОМОГАТЕЛЬНЫЕ КОНСТРУКТОРЫ / КОНФИГИ
// ---------------------------------------------------------

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

fn base_schedule() -> TournamentScheduleConfig {
    TournamentScheduleConfig {
        scheduled_start_ts: 1_000_000,
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

fn base_tournament_config(max_players: u32) -> TournamentConfig {
    TournamentConfig {
        name: "StressTournament".into(),
        description: None,
        starting_stack: Chips(10_000),
        max_players,
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

/// Создаёт турнир, регистрирует `player_count` игроков и запускает его (status = Running).
fn create_tournament_with_players(
    id: TournamentId,
    owner: PlayerId,
    player_count: u32,
) -> Tournament {
    assert!(player_count >= 2, "Для старта нужно >= 2 игроков");

    let mut cfg = base_tournament_config(player_count);
    cfg.max_players = player_count;

    let mut t = Tournament::new(id, owner, cfg)
        .expect("Tournament::new must succeed in stress tests");

    for i in 0..player_count {
        let pid: PlayerId = (1000 + i as u64) as PlayerId;
        t.register_player(pid)
            .expect("registration must succeed in stress tests");
    }

    let start_ts: u64 = 1_000_000;
    t.start(start_ts).expect("tournament start must succeed");
    assert_eq!(t.status, TournamentStatus::Running);
    assert_eq!(t.total_entries, t.active_player_count() as u32);

    t
}

// ---------------------------------------------------------
// ВСПОМОГАТЕЛЬНЫЕ РАНДОМНЫЕ ФУНКЦИИ ЧЕРЕЗ RandomSource::shuffle
// ---------------------------------------------------------

/// Выбор случайного индекса [0, len), используя только shuffle.
fn random_index<R: RandomSource>(rng: &mut R, len: usize) -> usize {
    assert!(len > 0);
    let mut indices: Vec<usize> = (0..len).collect();
    rng.shuffle(&mut indices);
    indices[0]
}

/// Случайное значение в диапазоне [0, choices.len()).
fn random_choice<R: RandomSource>(rng: &mut R, choices: &[u8]) -> u8 {
    assert!(!choices.is_empty());
    let mut arr = choices.to_vec();
    rng.shuffle(&mut arr);
    arr[0]
}

// ---------------------------------------------------------
// ИНВАРИАНТЫ ТУРНИРА
// ---------------------------------------------------------

fn assert_tournament_invariants(t: &Tournament) {
    // Специальный случай: турнир ещё в регистрации.
    if t.status == TournamentStatus::Registering {
        // В регистрации не должно быть финишировавших и победителя.
        assert_eq!(
            t.finished_count, 0,
            "В статусе Registering не должно быть финишировавших игроков"
        );
        assert!(
            t.winner_id.is_none(),
            "В статусе Registering не должно быть winner_id"
        );
        // total_entries и количество активных регистраций могут быть несогласованы:
        // total_entries обычно становиться >0 только после старта турнира.
        return;
    }

    let active: Vec<_> = t.active_players().collect();
    let active_count = active.len() as u32;

    // total_entries >= finished_count
    assert!(
        t.total_entries >= t.finished_count,
        "total_entries ({}) < finished_count ({})",
        t.total_entries,
        t.finished_count
    );

    // active + finished не должны быть больше total_entries
    assert!(
        t.total_entries >= active_count + t.finished_count,
        "total_entries ({}) < active ({}) + finished_count ({})",
        t.total_entries,
        active_count,
        t.finished_count
    );

    // finishing_place в [1, total_entries]
    for reg in t.registrations.values() {
        if let Some(place) = reg.finishing_place {
            assert!(
                place >= 1 && place <= t.total_entries,
                "finishing_place {} вне диапазона [1, {}]",
                place,
                t.total_entries
            );
        }
    }

    // Если Finished:
    if t.is_finished() {
        let active_after_finish: Vec<_> = t.active_players().collect();
        if active_after_finish.is_empty() {
            // Турнир мог закончиться без активных игроков — winner_id может быть None или Some.
        } else {
            // Есть активные игроки -> должен быть winner_id.
            assert!(
                t.winner_id.is_some(),
                "Finished турнир с активными игроками, но без winner_id"
            );
        }
    }
}

// ---------------------------------------------------------
// 1) 1000 шагов на одном турнире (умеренный стресс)
// ---------------------------------------------------------

#[test]
fn simulate_1000_steps_tournament_state_remains_consistent() {
    let owner: PlayerId = 1;
    let mut t = create_tournament_with_players(1, owner, 50);

    let mut rng = DeterministicRng::from_u64(12345);
    let mut now_ts: u64 = 1_000_000;

    for _step in 0..1000 {
        now_ts += 30;

        // тик по времени
        let _ = t.apply_time_tick(now_ts);

        // иногда выбиваем случайного игрока (если >= 2 активных)
        let actives: Vec<_> = t.active_players().map(|r| r.player_id).collect();
        if actives.len() >= 2 {
            let idx = random_index(&mut rng, actives.len());
            let target = actives[idx];
            let _ = t.mark_player_busted(target);
        }

        // ребаланс
        let moves = t.compute_rebalance_moves();
        t.apply_rebalance_moves(&moves);

        // инварианты должны держаться
        assert_tournament_invariants(&t);

        if t.is_finished() {
            break;
        }
    }
}

// ---------------------------------------------------------
// 2) 50 полных турниров подряд
// ---------------------------------------------------------

#[test]
fn simulate_50_full_tournaments_finish_correctly() {
    let owner: PlayerId = 1;
    let mut rng = DeterministicRng::from_u64(9999);

    let mut tournaments: Vec<Tournament> = (0..50)
        .map(|i| {
            let tid: TournamentId = (10_000 + i) as TournamentId;
            create_tournament_with_players(tid, owner, 40)
        })
        .collect();

    for t in tournaments.iter_mut() {
        let mut steps = 0u32;
        let mut now_ts: u64 = 1_000_000;

        while !t.is_finished() && steps < 20_000 {
            steps += 1;
            now_ts += 30;
            let _ = t.apply_time_tick(now_ts);

            let actives: Vec<_> = t.active_players().map(|r| r.player_id).collect();
            if actives.len() >= 2 {
                let idx = random_index(&mut rng, actives.len());
                let target = actives[idx];
                let _ = t.mark_player_busted(target);
            }

            let moves = t.compute_rebalance_moves();
            t.apply_rebalance_moves(&moves);

            assert_tournament_invariants(t);
        }

        assert!(
            t.is_finished(),
            "Турнир id={} не завершился за {} шагов",
            t.id,
            steps
        );
        assert_tournament_invariants(t);
    }
}

// ---------------------------------------------------------
// 3) Random actions generator (регистрация / bust / time / rebalance)
// ---------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum RandomTournamentOp {
    RegisterNewPlayer,
    BustRandomPlayer,
    TimeTick,
    Rebalance,
    Noop,
}

fn random_tournament_op<R: RandomSource>(rng: &mut R) -> RandomTournamentOp {
    match random_choice(rng, &[0, 1, 2, 3, 4]) {
        0 => RandomTournamentOp::RegisterNewPlayer,
        1 => RandomTournamentOp::BustRandomPlayer,
        2 => RandomTournamentOp::TimeTick,
        3 => RandomTournamentOp::Rebalance,
        _ => RandomTournamentOp::Noop,
    }
}

#[test]
fn random_actions_generator_keeps_tournament_consistent() {
    let owner: PlayerId = 1;
    let mut cfg = base_tournament_config(500);
    cfg.max_players = 500;

    let mut t = Tournament::new(5000, owner, cfg).expect("Tournament::new must succeed");
    let mut rng = DeterministicRng::from_u64(42);
    let mut now_ts: u64 = 1_000_000;
    let mut next_player_id: PlayerId = 10_000;

    for _step in 0..5000 {
        let op = random_tournament_op(&mut rng);

        match op {
            RandomTournamentOp::RegisterNewPlayer => {
                if t.status == TournamentStatus::Registering
                    && (t.registrations.len() as u32) < t.config.max_players
                {
                    let pid = next_player_id;
                    next_player_id += 1;
                    let _ = t.register_player(pid);

                    // иногда стартуем турнир
                    if t.can_start_now(now_ts) {
                        let _ = t.start(now_ts);
                    }
                }
            }
            RandomTournamentOp::BustRandomPlayer => {
                if t.status == TournamentStatus::Running {
                    let actives: Vec<_> =
                        t.active_players().map(|r| r.player_id).collect();
                    if actives.len() >= 2 {
                        let idx = random_index(&mut rng, actives.len());
                        let target = actives[idx];
                        let _ = t.mark_player_busted(target);
                    }
                }
            }
            RandomTournamentOp::TimeTick => {
                now_ts += 60;
                let _ = t.apply_time_tick(now_ts);
            }
            RandomTournamentOp::Rebalance => {
                let moves = t.compute_rebalance_moves();
                t.apply_rebalance_moves(&moves);
            }
            RandomTournamentOp::Noop => {
                // ничего не делаем
            }
        }

        assert_tournament_invariants(&t);
        if t.is_finished() {
            break;
        }
    }
}

// ---------------------------------------------------------
// 4) БОЛЬШОЙ ТУРНИР: 1000 игроков, много шагов (heavy)       #[ignore]
// ---------------------------------------------------------

#[test]
#[ignore]
fn large_tournament_1000_players_many_steps_stays_consistent() {
    let owner: PlayerId = 1;
    let mut t = create_tournament_with_players(9000, owner, 1000);

    let mut rng = DeterministicRng::from_u64(7777);
    let mut now_ts: u64 = 1_000_000;

    let mut steps = 0u32;

    while !t.is_finished() && steps < 20_000 {
        steps += 1;
        now_ts += 30;

        // тик времени
        let _ = t.apply_time_tick(now_ts);

        // bust (если есть кого bust’ить)
        let actives: Vec<_> = t.active_players().map(|r| r.player_id).collect();
        if actives.len() >= 2 {
            let idx = random_index(&mut rng, actives.len());
            let target = actives[idx];
            let _ = t.mark_player_busted(target);
        }

        // ребаланс
        let moves = t.compute_rebalance_moves();
        t.apply_rebalance_moves(&moves);

        assert_tournament_invariants(&t);
    }

    assert!(
        t.is_finished(),
        "Большой турнир на 1000 игроков не завершился за {} шагов",
        steps
    );
    assert_tournament_invariants(&t);
}

// ---------------------------------------------------------
// 5) МНОГО ПАРАЛЛЕЛЬНЫХ БОЛЬШИХ ТУРНИРОВ (heavy)            #[ignore]
// ---------------------------------------------------------

#[test]
#[ignore]
fn many_parallel_large_tournaments_finish_correctly() {
    let owner: PlayerId = 1;
    let mut rng = DeterministicRng::from_u64(13579);

    // 10 больших турниров по 300 игроков
    let mut tournaments: Vec<Tournament> = (0..10)
        .map(|i| {
            let tid: TournamentId = (20_000 + i) as TournamentId;
            create_tournament_with_players(tid, owner, 300)
        })
        .collect();

    let mut global_steps = 0u32;
    let max_global_steps = 50_000u32;

    while global_steps < max_global_steps {
        global_steps += 1;

        // Выбираем случайный турнир
        let unfinished_indices: Vec<usize> = tournaments
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.is_finished())
            .map(|(idx, _)| idx)
            .collect();

        if unfinished_indices.is_empty() {
            break; // все закончились
        }

        let idx = random_index(&mut rng, unfinished_indices.len());
        let tidx = unfinished_indices[idx];
        let t = &mut tournaments[tidx];

        // Локальное время для этого турнира
        let now_ts: u64 = 1_000_000 + global_steps as u64;
        let _ = t.apply_time_tick(now_ts);

        // Bust случайного игрока (если можно)
        let actives: Vec<_> = t.active_players().map(|r| r.player_id).collect();
        if actives.len() >= 2 {
            let pidx = random_index(&mut rng, actives.len());
            let target = actives[pidx];
            let _ = t.mark_player_busted(target);
        }

        let moves = t.compute_rebalance_moves();
        t.apply_rebalance_moves(&moves);

        assert_tournament_invariants(t);
    }

    // Все турниры должны быть завершены
    for t in tournaments.iter() {
        assert!(
            t.is_finished(),
            "Один из параллельных турниров (id={}) не завершился",
            t.id
        );
        assert_tournament_invariants(t);
    }
}
