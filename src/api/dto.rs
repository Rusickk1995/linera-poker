use serde::{Deserialize, Serialize};

use crate::domain::card::Card;
use crate::domain::chips::Chips;
use crate::domain::hand::{HandRank, Street};
use crate::domain::player::PlayerStatus;
use crate::domain::{PlayerId, TableId, TournamentId};
use crate::engine::HandStatus;

/// DTO игрока за столом.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerAtTableDto {
    pub player_id: PlayerId,
    pub display_name: String,
    pub seat_index: u8,
    pub stack: Chips,
    pub current_bet: Chips,
    pub status: PlayerStatus,
    /// Карманные карты – только для "героя" или в режиме администратора.
    pub hole_cards: Option<Vec<Card>>,
}

/// DTO стола.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableViewDto {
    pub table_id: TableId,
    pub name: String,
    pub max_seats: u8,
    pub small_blind: Chips,
    pub big_blind: Chips,
    pub ante: Chips,
    pub street: Street,
    pub dealer_button: Option<u8>,
    pub total_pot: Chips,
    pub board: Vec<Card>,
    pub players: Vec<PlayerAtTableDto>,
    /// Есть ли активная раздача.
    pub hand_in_progress: bool,
    /// Текущий игрок, чей ход (если раздача идёт).
    pub current_actor_seat: Option<u8>,
}

/// DTO одной сыгранной раздачи (для истории).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandHistoryItemDto {
    pub hand_id: u64,
    pub street_reached: Street,
    pub board: Vec<Card>,
    pub total_pot: Chips,
    /// Для каждого игрока – что он выиграл/проиграл.
    pub players: Vec<HandPlayerResultDto>,
}

/// Результат одного игрока в раздаче.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandPlayerResultDto {
    pub player_id: PlayerId,
    pub seat_index: u8,
    pub net_chips: Chips,
    pub is_winner: bool,
    pub rank: Option<HandRank>,
}

/// DTO турнира (минимальное представление для лобби/ончейна).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TournamentViewDto {
    pub tournament_id: TournamentId,
    pub name: String,
    /// Статус в текстовом виде: "Registering", "Running", "Finished" и т.п.
    pub status: String,
    pub current_level: u32,
    pub players_registered: u32,
    pub tables_running: u32,
}

/// Ответ API на команду.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CommandResponse {
    /// Успешный результат без доп.данных.
    Ok,

    /// Вернуть обновлённое состояние стола.
    TableState(TableViewDto),

    /// Вернуть результат раздачи (когда HandStatus::Finished).
    HandFinished {
        table: TableViewDto,
        history: Option<HandHistoryItemDto>,
    },

    /// Создан новый стол.
    TableCreated(TableViewDto),

    /// Состояние турнира после турнирной команды.
    TournamentState(TournamentViewDto),
}

/// Помощник: преобразование HandStatus движка в DTO.
pub fn map_hand_status_to_response(
    status: HandStatus,
    table_dto: TableViewDto,
) -> CommandResponse {
    match status {
        HandStatus::Ongoing => CommandResponse::TableState(table_dto),

        HandStatus::Finished(summary, _history) => {
            // Быстрый индекс: PlayerId -> seat_index из актуального TableViewDto.
            let mut seat_by_player: std::collections::HashMap<PlayerId, u8> =
                std::collections::HashMap::new();

            for p in &table_dto.players {
                seat_by_player.insert(p.player_id, p.seat_index);
            }

            let players = summary
                .results
                .into_iter()
                .map(|r| {
                    let seat_index = seat_by_player.get(&r.player_id).copied().unwrap_or(255);

                    HandPlayerResultDto {
                        player_id: r.player_id,
                        seat_index,
                        net_chips: r.net_chips,
                        is_winner: r.is_winner,
                        rank: r.rank,
                    }
                })
                .collect();

            let hist = HandHistoryItemDto {
                hand_id: summary.hand_id,
                street_reached: summary.street_reached,
                board: summary.board,
                total_pot: summary.total_pot,
                players,
            };

            CommandResponse::HandFinished {
                table: table_dto,
                history: Some(hist),
            }
        }
    }
}
