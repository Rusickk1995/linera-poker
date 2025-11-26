// src/tournament/runtime.rs

use crate::domain::chips::Chips;
use crate::domain::player::PlayerAtTable;
use crate::domain::table::{Table, TableConfig, TableStakes, TableType};
use crate::domain::blinds::AnteType;
use crate::domain::tournament::{PlayerRegistration, Tournament};
use crate::domain::{PlayerId, TableId, TournamentId};

/// Посадка игрока за конкретный турнирный стол.
#[derive(Clone, Debug)]
pub struct TournamentTableSeat {
    /// Игрок.
    pub player_id: PlayerId,
    /// Индекс места за столом (0-based).
    pub seat_index: u8,
    /// Текущий стек игрока в фишках.
    pub stack: Chips,
    /// Сколько раз игрок уже заходил в турнир (1 + ре-энтри).
    pub entries_used: u32,
}

/// Один турнирный стол в рантайме:
/// - реальный `Table` с конфигом;
/// - список посадок игроков (для удобства дебага / фронта).
#[derive(Clone, Debug)]
pub struct TournamentTableInstance {
    /// Идентификатор турнира.
    pub tournament_id: TournamentId,
    /// Сам стол (доменная структура покерного стола).
    pub table: Table,
    /// Посадка игроков за этим столом.
    pub seats: Vec<TournamentTableSeat>,
}

/// Вспомогательный рантайм-слой для турнира.
/// Сейчас он умеет:
/// - брать зарегистрированных игроков турнира;
/// - раскладывать их по столам по `table_size`;
/// - для каждого стола собирать `TableConfig` и `Table` с посаженными игроками.
///
/// ВНИМАНИЕ: здесь пока нет HandEngine и логики сдачи/розыгрыша.
/// Это будет следующим шагом поверх этих столов.
pub struct TournamentRuntime;

impl TournamentRuntime {
    /// Построить список турнирных столов для конкретного турнира.
    pub fn build_tables_for_tournament(tournament: &Tournament) -> Vec<TournamentTableInstance> {
        // 1. Собираем регистрации в вектор и сортируем для детерминированности.
        let mut regs: Vec<&PlayerRegistration> =
            tournament.registrations_iter().map(|(_, reg)| reg).collect();

        regs.sort_by_key(|reg| reg.player_id);

        if regs.is_empty() {
            return Vec::new();
        }

        // 2. Размер стола (гарантируем минимум 2, чтобы не делить на 0).
        let table_size: u8 = tournament.config.table_size.max(2);

        // 3. Базовая структура ставок для турнирного стола.
        //
        // На этом уровне у нас ещё нет детальной структуры блайндов по уровням,
        // поэтому берём "дефолтные" блайнды.
        // Позже сюда можно будет подставить значения из TournamentConfig.
        let stakes = TableStakes::new(
            Chips::new(50),           // small blind
            Chips::new(100),          // big blind
            AnteType::None,           // анте отсутствует
            Chips::ZERO,              // страддл по умолчанию 0
        );

        // Базовый конфиг стола для турнира.
        let base_config = TableConfig {
            max_seats: table_size,
            table_type: TableType::Tournament,
            stakes,
            allow_straddle: false,
            allow_run_it_twice: false,
        };

        // 4. Чанкуем список игроков по `table_size` и на каждый чанк создаём стол.
        let mut result = Vec::new();
        let mut local_table_index: u64 = 0;

        for chunk in regs.chunks(table_size as usize) {
            local_table_index += 1;

            // Временно создаём локальный идентификатор стола.
            // В дальнейшем можно будет использовать более сложную схему,
            // либо брать уже существующие TableId из другого слоя.
            let table_id: TableId = local_table_index as TableId;

            // Имя стола: просто "T{tournament_id}-Table{n}".
            let table_name = format!("T{}-Table{}", tournament.id, local_table_index);

            // Клон конфига для конкретного стола.
            let config = base_config.clone();

            // Собираем посадку.
            let mut seats = Vec::new();
            for (seat_index, reg) in chunk.iter().enumerate() {
                seats.push(TournamentTableSeat {
                    player_id: reg.player_id,
                    seat_index: seat_index as u8,
                    stack: tournament.config.starting_stack,
                    entries_used: reg.entries_used,
                });
            }

            // Создаём доменный стол и заполняем его сидения.
            let mut table = Table::new(table_id, table_name, config.clone());

            // Инициализируем все места пустыми, если их ещё нет.
            if table.seats.len() < table_size as usize {
                table.seats.resize(table_size as usize, None);
            }

            for seat in &seats {
                let idx = seat.seat_index as usize;
                if idx >= table.seats.len() {
                    continue;
                }
                table.seats[idx] = Some(PlayerAtTable::new(seat.player_id, seat.stack));
            }

            result.push(TournamentTableInstance {
                tournament_id: tournament.id,
                table,
                seats,
            });
        }

        result
    }
}
