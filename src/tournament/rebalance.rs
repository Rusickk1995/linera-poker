use std::collections::HashMap;

use crate::domain::{PlayerId, TableId};

/// Перемещение одного игрока между столами при ребалансировке.
///
/// В реальном рантайме ты:
///   1) применяешь это к турнирному состоянию (обновляешь table_id / seat_index),
///   2) пересаживаешь игрока в движке столов (engine).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebalanceMove {
    pub player_id: PlayerId,
    pub from_table: TableId,
    pub to_table: TableId,
}

/// Снимок одного стола: кто за ним сейчас сидит.
///
/// Можно использовать, если хочешь дебажить/логировать
/// или визуализировать состояние до/после ребалансировки.
#[derive(Clone, Debug)]
pub struct TableOccupancy {
    pub table_id: TableId,
    pub players: Vec<PlayerId>,
}

/// Полный план ребалансировки:
///   - список перемещений,
///   - итоговое распределение игроков по столам
///     после применения всех перемещений.
#[derive(Clone, Debug)]
pub struct RebalancePlan {
    /// Последовательность шагов, которые нужно выполнить:
    /// перенести player_id с from_table на to_table.
    pub moves: Vec<RebalanceMove>,

    /// Итоговое распределение игроков по столам после выполнения moves.
    ///
    /// Ключ: table_id
    /// Значение: список player_id, уже с учётом всех перемещений.
    pub final_distribution: HashMap<TableId, Vec<PlayerId>>,
}

/// Проверка, сбалансированы ли столы по количеству игроков.
///
/// Правило:
///   - считаем min и max кол-во игроков среди всех столов;
///   - если (max - min) <= max_seat_diff — считаем расклад допустимым.
pub fn is_balanced(
    tables: &HashMap<TableId, Vec<PlayerId>>,
    max_seat_diff: u8,
) -> bool {
    if tables.len() <= 1 {
        return true;
    }

    let mut min_count: Option<usize> = None;
    let mut max_count: Option<usize> = None;

    for players in tables.values() {
        let c = players.len();
        min_count = Some(min_count.map_or(c, |m| m.min(c)));
        max_count = Some(max_count.map_or(c, |m| m.max(c)));
    }

    match (min_count, max_count) {
        (Some(min), Some(max)) => max.saturating_sub(min) <= max_seat_diff as usize,
        _ => true,
    }
}

/// Основной алгоритм ребалансировки столов.
///
/// Вход:
///   - original_tables: снимок текущего состояния
///       table_id -> список player_id за этим столом.
///   - max_seat_diff: максимально допустимая разница по кол-ву игроков
///       между любыми двумя столами (обычно 1 или 2).
///
/// Выход:
///   - RebalancePlan:
///       * moves: последовательность RebalanceMove,
///       * final_distribution: итоговое распределение игроков.
///
/// Алгоритм (классическая схема, как делают нормальные студии):
///   1. Берём стол с максимальным кол-вом игроков (донор),
///      и стол с минимальным кол-вом игроков (реципиент).
///   2. Если разница между max и min уже <= max_seat_diff — стоп.
///   3. Иначе переносим одного игрока с донора на реципиента.
///   4. Повторяем, пока все столы не удовлетворяют условию.
pub fn compute_rebalance_plan(
    original_tables: &HashMap<TableId, Vec<PlayerId>>,
    max_seat_diff: u8,
) -> RebalancePlan {
    // Если один стол или max_seat_diff = 0 — ничего не делаем.
    if original_tables.len() <= 1 || max_seat_diff == 0 {
        return RebalancePlan {
            moves: Vec::new(),
            final_distribution: original_tables.clone(),
        };
    }

    // Копируем распределение, чтобы не трогать оригинал.
    let mut distribution: HashMap<TableId, Vec<PlayerId>> = original_tables.clone();

    // Для детерминизма сортируем игроков по id и убираем дубликаты.
    for players in distribution.values_mut() {
        players.sort();
        players.dedup();
    }

    let mut moves: Vec<RebalanceMove> = Vec::new();

    loop {
        // Сортируем столы по table_id, чтобы выбор всегда был детерминированным.
        let mut table_ids: Vec<TableId> = distribution.keys().cloned().collect();
        table_ids.sort();

        let mut min_table: Option<(TableId, usize)> = None;
        let mut max_table: Option<(TableId, usize)> = None;

        for tid in &table_ids {
            let count = distribution
                .get(tid)
                .map(|v| v.len())
                .unwrap_or(0);

            match min_table {
                None => min_table = Some((*tid, count)),
                Some((_, current_min)) if count < current_min => {
                    min_table = Some((*tid, count));
                }
                _ => {}
            }

            match max_table {
                None => max_table = Some((*tid, count)),
                Some((_, current_max)) if count > current_max => {
                    max_table = Some((*tid, count));
                }
                _ => {}
            }
        }

        let (max_id, max_cnt) = match max_table {
            Some(v) => v,
            None => break,
        };

        let (min_id, min_cnt) = match min_table {
            Some(v) => v,
            None => break,
        };

        // Если все столы равны по кол-ву игроков — всё уже ок.
        if max_id == min_id {
            break;
        }

        // Нет смысла двигать, если донор пустой.
        if max_cnt == 0 {
            break;
        }

        // Уже укладываемся в допустимую разницу — стоп.
        if max_cnt.saturating_sub(min_cnt) <= max_seat_diff as usize {
            break;
        }

        // Берём игрока с "набитого" стола.
        let from_vec = distribution
            .get_mut(&max_id)
            .expect("table must exist in distribution");
        let player_id = from_vec
            .pop()
            .expect("non-empty table must contain at least one player");

        // Пересаживаем его на "пустой" стол.
        let to_vec = distribution
            .get_mut(&min_id)
            .expect("table must exist in distribution");
        to_vec.push(player_id);

        // Фиксируем перемещение.
        moves.push(RebalanceMove {
            player_id,
            from_table: max_id,
            to_table: min_id,
        });
    }

    RebalancePlan {
        moves,
        final_distribution: distribution,
    }
}

/// Утилита для удобства: конвертировать распределение
/// в список TableOccupancy (можно использовать в логах, дебаге или тестах).
pub fn snapshot_tables(
    tables: &HashMap<TableId, Vec<PlayerId>>,
) -> Vec<TableOccupancy> {
    let mut res: Vec<TableOccupancy> = tables
        .iter()
        .map(|(tid, players)| TableOccupancy {
            table_id: *tid,
            players: players.clone(),
        })
        .collect();

    // Для стабильного порядка в логах сортируем по table_id.
    res.sort_by_key(|t| t.table_id);
    res
}
