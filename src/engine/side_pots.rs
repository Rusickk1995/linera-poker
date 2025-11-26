use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::{chips::Chips, SeatIndex};

/// Сайд-пот: часть банка, в которую участвуют только некоторые игроки.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidePot {
    pub amount: Chips,
    pub eligible_seats: Vec<SeatIndex>,
}

/// Посчитать сайд-поты из сумм, которые внесли игроки.
///
/// Вход: contributions[seat] = сколько суммарно фишек поставил игрок (во всех улицах).
/// Выход: список side pots в порядке "от младших" к "старшим".
pub fn compute_side_pots(contributions: &HashMap<SeatIndex, Chips>) -> Vec<SidePot> {
    // Собираем (seat, amount > 0)
    let mut entries: Vec<(SeatIndex, Chips)> = contributions
        .iter()
        .filter_map(|(seat, chips)| {
            if chips.is_zero() {
                None
            } else {
                Some((*seat, *chips))
            }
        })
        .collect();

    if entries.is_empty() {
        return Vec::new();
    }

    // Сортируем по размеру вклада (возрастание).
    entries.sort_by_key(|(_, c)| c.0);

    let mut pots = Vec::new();
    let mut prev_level = Chips::ZERO;

    for (i, &(_, amount)) in entries.iter().enumerate() {
        if amount == prev_level {
            continue;
        }
        let level_diff = amount - prev_level;

        // Все игроки, у кого вклад >= amount, участвуют в этом уровне.
        let mut eligible = Vec::new();
        for (seat, contrib) in entries.iter() {
            if contrib.0 >= amount.0 {
                eligible.push(*seat);
            }
        }

        if !eligible.is_empty() {
            // Размер сайд-пота = diff * число игроков, которые хотя бы на этом уровне.
            let pot_amount = Chips(level_diff.0 * eligible.len() as u64);
            pots.push(SidePot {
                amount: pot_amount,
                eligible_seats: eligible,
            });
        }

        prev_level = amount;
    }

    pots
}
