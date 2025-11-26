use crate::api::{AnteTypeApi, dto::TableViewDto};
use crate::domain::blinds::AnteType;
use crate::domain::player::{PlayerAtTable, PlayerStatus};
use crate::domain::table::Table;
use crate::domain::PlayerId;
use crate::engine::game_loop::HandEngine;

/// Маппинг типа анте между API и domain.
pub fn ante_type_from_api(api: AnteTypeApi) -> AnteType {
    match api {
        AnteTypeApi::None => AnteType::None,
        AnteTypeApi::Classic => AnteType::Classic,
        AnteTypeApi::BigBlind => AnteType::BigBlind,
    }
}

pub fn ante_type_to_api(domain: AnteType) -> AnteTypeApi {
    match domain {
        AnteType::None => AnteTypeApi::None,
        AnteType::Classic => AnteTypeApi::Classic,
        AnteType::BigBlind => AnteTypeApi::BigBlind,
    }
}

/// Утилита: получить отображаемое имя игрока.
///
/// В on-chain варианте это делается через `PokerState::player_names`,
/// но здесь оставляем сигнатуру, которую можно реализовать по-разному.
pub trait PlayerNameResolver {
    fn resolve_name(&self, player_id: PlayerId) -> String;
}

/// Простая реализация: отображаемое имя = "Player {id}".
pub struct DefaultNameResolver;

impl PlayerNameResolver for DefaultNameResolver {
    fn resolve_name(&self, player_id: PlayerId) -> String {
        format!("Player {}", player_id)
    }
}

/// Утилита: маппинг Table + HandEngine -> TableViewDto.
/// По сути это то же, что `api::queries::build_table_view`, вынесенное в infra.
///
/// Можно использовать и off-chain, и on-chain, если удобно.
pub fn map_table_to_dto(
    table: &Table,
    engine: Option<&HandEngine>,
    name_resolver: &impl PlayerNameResolver,
    is_hero: impl Fn(PlayerId) -> bool,
) -> TableViewDto {
    use crate::api::dto::PlayerAtTableDto;

    let mut players_dto = Vec::new();

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

            players_dto.push(PlayerAtTableDto {
                player_id: pid,
                display_name: name_resolver.resolve_name(pid),
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
        players: players_dto,
        hand_in_progress: table.hand_in_progress,
        current_actor_seat,
    }
}

/// Простейшая проверка, является ли seat "активным" за столом.
pub fn is_seat_active(table: &Table, seat_index: usize) -> bool {
    table
        .seats
        .get(seat_index)
        .and_then(|s| s.as_ref())
        .map(|p| matches!(p.status, PlayerStatus::Active | PlayerStatus::AllIn))
        .unwrap_or(false)
}
