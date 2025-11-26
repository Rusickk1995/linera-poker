use crate::domain::{SeatIndex, Table};

/// Найти следующее занятое место по кругу (включая/исключая start).
pub fn next_occupied_seat(table: &Table, start: SeatIndex, include_start: bool) -> Option<SeatIndex> {
    if table.seats.is_empty() {
        return None;
    }

    let max = table.max_seats() as usize;
    let mut idx = start as usize;

    if !include_start {
        idx = (idx + 1) % max;
    }

    for _ in 0..max {
        if idx < table.seats.len() && table.seats[idx].is_some() {
            return Some(idx as SeatIndex);
        }
        idx = (idx + 1) % max;
    }

    None
}

/// Найти n активных мест начиная с seat (по кругу).
pub fn collect_occupied_seats_from(table: &Table, start: SeatIndex) -> Vec<SeatIndex> {
    let max = table.max_seats() as usize;
    let mut seats = Vec::new();

    if max == 0 {
        return seats;
    }

    let mut idx = start as usize;
    for _ in 0..max {
        if idx < table.seats.len() && table.seats[idx].is_some() {
            seats.push(idx as SeatIndex);
        }
        idx = (idx + 1) % max;
    }

    seats
}

/// Предложить следующую позицию дилера:
/// - если есть текущая кнопка – следующая занятая.
/// - если нет – ищем первую занятую.
pub fn next_dealer(table: &Table) -> Option<SeatIndex> {
    if let Some(button) = table.dealer_button {
        next_occupied_seat(table, button, false)
    } else {
        // Ищем первый занятый seat с 0.
        next_occupied_seat(table, 0, true)
    }
}
