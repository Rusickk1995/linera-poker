use serde::{Deserialize, Serialize};

use crate::domain::player::PlayerAtTable;
use crate::domain::table::Table;
use crate::domain::{PlayerId, TableId, TournamentId};
use crate::engine::HandEngine;

use super::dto::{PlayerAtTableDto, TableViewDto};

/// Запросы "только чтение".
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Query {
    /// Получить состояние стола.
    GetTable { table_id: TableId },

    /// Получить список столов (для лобби).
    ListTables,

    /// Получить минимальную инфу о турнире.
    GetTournament { tournament_id: TournamentId },
}

/// Результат запроса "только чтение".
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueryResponse {
    Table(TableViewDto),
    Tables(Vec<TableViewDto>),
    TournamentInfo(super::dto::TournamentViewDto),
}

/// Сформировать DTO стола на основе `Table` + опционального `HandEngine`.
/// `current_actor_seat` берём из engine, если он есть.
pub fn build_table_view(
    table: &Table,
    engine: Option<&HandEngine>,
    resolve_name: impl Fn(PlayerId) -> String,
    is_hero: impl Fn(PlayerId) -> bool,
) -> TableViewDto {
    let players = build_players_dto(table, &resolve_name, &is_hero);

    let current_actor_seat = engine
        .and_then(|e| e.current_actor)
        .map(|s| s as u8);

    TableViewDto {
        table_id: table.id,
        name: table.name.clone(),
        max_seats: table.config.max_seats,
        small_blind: table.config.stakes.small_blind,
        big_blind: table.config.stakes.big_blind,
        ante: table.config.stakes.ante,
        street: table.street,
        dealer_button: table.dealer_button.map(|s| s as u8),
        total_pot: table.total_pot,
        board: table.board.clone(),
        players,
        hand_in_progress: table.hand_in_progress,
        current_actor_seat,
    }
}

/// Собрать DTO игроков за столом.
fn build_players_dto(
    table: &Table,
    resolve_name: &impl Fn(PlayerId) -> String,
    is_hero: &impl Fn(PlayerId) -> bool,
) -> Vec<PlayerAtTableDto> {
    let mut res = Vec::new();

    for (idx, seat_opt) in table.seats.iter().enumerate() {
        if let Some(PlayerAtTable {
            player_id,
            stack,
            current_bet,
            status,
            hole_cards,
        }) = seat_opt
        {
            let pid = *player_id;
            let show_cards = is_hero(pid);

            res.push(PlayerAtTableDto {
                player_id: pid,
                display_name: resolve_name(pid),
                seat_index: idx as u8,
                stack: *stack,
                current_bet: *current_bet,
                status: *status,
                hole_cards: if show_cards {
                    Some(hole_cards.clone())
                } else {
                    None
                },
            });
        }
    }

    res
}
