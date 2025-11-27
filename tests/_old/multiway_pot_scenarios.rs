// tests/multiway_pot_scenarions.rs

use poker_engine::domain::chips::Chips;
use poker_engine::domain::SeatIndex;
use poker_engine::engine::side_pots::{compute_side_pots, SidePot};

use std::collections::HashMap;

/// Хелпер: удобное создание contributions карты.
fn contrib(entries: &[(SeatIndex, u64)]) -> HashMap<SeatIndex, Chips> {
    let mut m = HashMap::new();
    for (seat, amount) in entries {
        m.insert(*seat, Chips(*amount));
    }
    m
}

/// Сценарий:
/// - Seat 0 вложил 100
/// - Seat 1 вложил 200
/// - Seat 2 вложил 400
///
/// Ожидаем:
/// - Main pot: 100 * 3 = 300, участвуют [0,1,2]
/// - Side pot 1: (200-100) * 2 = 200, участвуют [1,2]
/// - Side pot 2: (400-200) * 1 = 200, участвует [2]
#[test]
fn multiway_all_in_side_pots_are_computed_correctly() {
    let contributions = contrib(&[(0, 100), (1, 200), (2, 400)]);

    let pots: Vec<SidePot> = compute_side_pots(&contributions);
    assert_eq!(pots.len(), 3, "Должно быть 3 pot-а (main + 2 side)");

    // Для удобства сортируем по amount
    let mut pots_sorted = pots.clone();
    pots_sorted.sort_by_key(|p| p.amount.0);

    // Проверяем суммы
    assert_eq!(pots_sorted[0].amount.0, 200); // самый маленький: 200
    assert_eq!(pots_sorted[1].amount.0, 200);
    assert_eq!(pots_sorted[2].amount.0, 300);

    // Проверяем наборы eligible_seats (множества игроков, кто может претендовать на pot)
    let mut sets: Vec<Vec<SeatIndex>> = pots_sorted
        .iter()
        .map(|p| {
            let mut s = p.eligible_seats.clone();
            s.sort();
            s
        })
        .collect();

    sets.sort(); // сортируем сами наборы для стабильности проверок

    assert!(
        sets.contains(&vec![0, 1, 2]),
        "Должен быть pot для [0,1,2]"
    );
    assert!(sets.contains(&vec![1, 2]), "Должен быть pot для [1,2]");
    assert!(sets.contains(&vec![2]), "Должен быть pot для [2]");
}

/// Corner-case: если один игрок вложил 0, он не должен появляться в side pots.
#[test]
fn player_with_zero_contribution_is_not_in_pots() {
    let contributions = contrib(&[(0, 0), (1, 100), (2, 100)]);

    let pots = compute_side_pots(&contributions);
    assert_eq!(pots.len(), 1, "Должен быть один основной pot");

    let pot = &pots[0];
    assert_eq!(pot.amount.0, 200);
    let mut seats = pot.eligible_seats.clone();
    seats.sort();
    assert_eq!(seats, vec![1, 2], "Seat 0 не должен участвовать");
}
