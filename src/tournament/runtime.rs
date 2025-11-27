// src/tournament/runtime.rs

use crate::domain::blinds::AnteType;
use crate::domain::chips::Chips;
use crate::domain::player::PlayerAtTable;
use crate::domain::table::{Table, TableConfig, TableStakes, TableType};
use crate::domain::tournament::{PlayerRegistration, Tournament};
use crate::domain::{PlayerId, TableId, TournamentId};

/// Посадка игрока за конкретный турнирный стол (для фронта/инфры).
#[derive(Clone, Debug)]
pub struct TournamentTableSeat {
    pub player_id: PlayerId,
    pub seat_index: u8,
    pub stack: Chips,
}

/// Экземпляр турнирного стола, который можно отдать движку/фронту.
#[derive(Clone, Debug)]
pub struct TournamentTableInstance {
    pub tournament_id: TournamentId,
    pub table: Table,
    pub seats: Vec<TournamentTableSeat>,
}

/// Runtime-утилита для работы с турнирами (построение столов и т.п.).
pub struct TournamentRuntime;

impl TournamentRuntime {
    pub fn build_tables_for_tournament(
        tournament: &Tournament,
        next_table_id: TableId,
    ) -> Vec<TournamentTableInstance> {
        let blind_level = tournament.current_blind_level();

        let mut regs: Vec<&PlayerRegistration> = tournament
            .registrations
            .values()
            .filter(|reg| !reg.is_busted)
            .collect();

        regs.sort_by_key(|reg| reg.player_id);

        let table_size = tournament.config.table_size.max(2) as usize;

        let mut result = Vec::new();
        if regs.is_empty() {
            return result;
        }

        let mut table_id_counter = next_table_id;
        let mut idx = 0usize;

        while idx < regs.len() {
            let end = (idx + table_size).min(regs.len());
            let slice = &regs[idx..end];

            let stakes = TableStakes::new(
                blind_level.small_blind,
                blind_level.big_blind,
                match blind_level.ante_type {
                    AnteType::None => AnteType::None,
                    AnteType::Classic => AnteType::Classic,
                    AnteType::BigBlind => AnteType::BigBlind,
                },
                blind_level.ante,
            );

            let table_config = TableConfig {
                max_seats: tournament.config.table_size,
                table_type: TableType::Tournament,
                stakes,
                allow_straddle: false,
                allow_run_it_twice: false,
            };

            let table_id = table_id_counter;
            table_id_counter += 1;

            let mut table = Table::new(
                table_id,
                format!("T#{} Table {}", tournament.id, table_id),
                table_config,
            );

            let mut seats_meta = Vec::with_capacity(slice.len());

            for (seat_index, reg) in slice.iter().enumerate() {
                let s_idx = seat_index as u8;
                let pat = PlayerAtTable::new(reg.player_id, reg.total_chips);

                if let Some(slot) = table.seats.get_mut(seat_index) {
                    *slot = Some(pat);
                }

                seats_meta.push(TournamentTableSeat {
                    player_id: reg.player_id,
                    seat_index: s_idx,
                    stack: reg.total_chips,
                });
            }

            result.push(TournamentTableInstance {
                tournament_id: tournament.id,
                table,
                seats: seats_meta,
            });

            idx = end;
        }

        result
    }
}
