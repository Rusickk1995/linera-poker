// tests/heads_up_scenarios.rs

use poker_engine::domain::blinds::AnteType;
use poker_engine::domain::chips::Chips;
use poker_engine::domain::hand::Street;
use poker_engine::domain::player::{PlayerAtTable, PlayerStatus};
use poker_engine::domain::table::{Table, TableConfig, TableStakes, TableType};
use poker_engine::domain::{HandId, PlayerId, SeatIndex, TableId};
use poker_engine::engine::actions::{PlayerAction, PlayerActionKind};
use poker_engine::engine::errors::EngineError;
use poker_engine::engine::game_loop::{apply_action, start_hand, HandEngine, HandStatus};
use poker_engine::engine::RandomSource;

use std::collections::HashMap;

/// Простой детерминированный RNG для тестов.
/// Нам не важно качество случайности — только чтобы раздачи были повторяемыми.
struct DummyRng(u64);

impl DummyRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Внутренний генератор 64-битного числа (LCG).
    fn next_u64(&mut self) -> u64 {
        // Очень простой LCG — для тестов более чем достаточно.
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        self.0
    }
}

/// Реализация трейта RandomSource под твой движок:
/// движку нужен только метод shuffle.
impl RandomSource for DummyRng {
    fn shuffle<T>(&mut self, slice: &mut [T]) {
        // Детерминированный Fisher–Yates, использующий next_u64().
        let len = slice.len();
        if len <= 1 {
            return;
        }

        for i in (1..len).rev() {
            let rnd = self.next_u64();
            let j = (rnd % (i as u64 + 1)) as usize;
            slice.swap(i, j);
        }
    }
}

/// Хелпер: создаём Heads-Up стол с двумя игроками.
fn create_heads_up_table() -> (Table, PlayerId, PlayerId) {
    let table_id: TableId = 1;
    let p1_id: PlayerId = 1;
    let p2_id: PlayerId = 2;

    let stakes = TableStakes {
        small_blind: Chips(50),
        big_blind: Chips(100),
        ante_type: AnteType::None,
        ante: Chips::ZERO,
    };

    let config = TableConfig {
        max_seats: 9,
        table_type: TableType::Cash,
        stakes,
        allow_straddle: false,
        allow_run_it_twice: false,
    };

    let mut seats: Vec<Option<PlayerAtTable>> = vec![None; config.max_seats as usize];

    seats[0] = Some(PlayerAtTable {
        player_id: p1_id,
        stack: Chips(10_000),
        current_bet: Chips::ZERO,
        status: PlayerStatus::Active,
        hole_cards: Vec::new(),
    });

    seats[1] = Some(PlayerAtTable {
        player_id: p2_id,
        stack: Chips(10_000),
        current_bet: Chips::ZERO,
        status: PlayerStatus::Active,
        hole_cards: Vec::new(),
    });

    let table = Table {
        id: table_id,
        name: "HU Test Table".to_string(),
        config,
        seats,
        dealer_button: None,
        board: Vec::new(),
        total_pot: Chips::ZERO,
        current_hand_id: None,
        hand_in_progress: false,
        street: Street::Preflop,
    };

    (table, p1_id, p2_id)
}

/// Тест: старт новой раздачи на HU-столе.
#[test]
fn heads_up_start_hand_initializes_state_correctly() -> Result<(), EngineError> {
    let (mut table, p1_id, p2_id) = create_heads_up_table();

    let mut rng = DummyRng::new(12345);
    let hand_id: HandId = 1;

    let engine: HandEngine = start_hand(&mut table, &mut rng, hand_id)?;

    assert!(table.hand_in_progress, "Раздача должна быть в процессе");
    assert_eq!(table.current_hand_id, Some(hand_id));

    assert!(
        table.dealer_button.is_some(),
        "Дилер (dealer_button) должен быть установлен"
    );

    let seats_with_players: Vec<(SeatIndex, &PlayerAtTable)> = table
        .seats
        .iter()
        .enumerate()
        .filter_map(|(idx, seat)| seat.as_ref().map(|p| (idx as SeatIndex, p)))
        .collect();

    assert_eq!(seats_with_players.len(), 2, "Должно быть ровно 2 игрока");

    for (_seat, player) in &seats_with_players {
        assert_eq!(
            player.hole_cards.len(),
            2,
            "У каждого игрока должно быть по 2 hole-карты"
        );
        assert!(
            player.player_id == p1_id || player.player_id == p2_id,
            "Ожидались игроки p1 и p2"
        );
    }

    assert!(
        engine.current_actor.is_some(),
        "current_actor должен быть установлен"
    );

    Ok(())
}

/// Тест: Fold в HU → раздача заканчивается и выигрывает второй игрок.
#[test]
fn heads_up_fold_finishes_hand_and_awards_pot() -> Result<(), EngineError> {
    let (mut table, _p1_id, _p2_id) = create_heads_up_table();

    let mut rng = DummyRng::new(999);
    let hand_id: HandId = 42;

    let mut engine = start_hand(&mut table, &mut rng, hand_id)?;

    let mut stacks_before: HashMap<PlayerId, Chips> = HashMap::new();
    for seat_opt in &table.seats {
        if let Some(p) = seat_opt.as_ref() {
            stacks_before.insert(p.player_id, p.stack);
        }
    }

    let acting_seat: SeatIndex = engine
        .current_actor
        .expect("current_actor должен быть установлен");
    let acting_player = table.seats[acting_seat as usize]
        .as_ref()
        .expect("На текущем seat должен сидеть игрок");
    let folding_player_id = acting_player.player_id;

    let action = PlayerAction {
        player_id: folding_player_id,
        seat: acting_seat,
        kind: PlayerActionKind::Fold,
    };

    let result = apply_action(&mut table, &mut engine, action)?;

    let (summary, _history) = match result {
        HandStatus::Ongoing => panic!("Ожидалась завершённая раздача после Fold в HU"),
        HandStatus::Finished(summary, history) => (summary, history),
    };

    assert!(
        summary.total_pot.0 > 0,
        "total_pot должен быть > 0 к моменту завершения"
    );

    let winners: Vec<_> = summary
        .results
        .iter()
        .filter(|r| r.is_winner)
        .collect();
    assert_eq!(winners.len(), 1, "В HU без шоудауна должен быть один победитель");
    let winner = winners[0];

    assert_ne!(
        winner.player_id, folding_player_id,
        "Победитель не должен быть тем, кто сделал Fold"
    );

    let winner_before = stacks_before
        .get(&winner.player_id)
        .expect("Должен быть сохранён стек победителя до раздачи");
    assert!(
        winner.net_chips.0 > 0,
        "net_chips победителя должен быть > 0"
    );

    let winner_after_stack = table
        .seats
        .iter()
        .filter_map(|s| s.as_ref())
        .find(|p| p.player_id == winner.player_id)
        .map(|p| p.stack)
        .expect("Не нашли стек победителя после раздачи");

    assert_eq!(
        winner_before.0 + winner.net_chips.0,
        winner_after_stack.0,
        "Стек победителя должен увеличиться на net_chips"
    );

    Ok(())
}
