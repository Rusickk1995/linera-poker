use std::collections::HashMap;

use linera_sdk::views::{linera_views, MapView, RegisterView, RootView, ViewStorageContext};
use serde::{Deserialize, Serialize};

use crate::domain::chips::Chips;
use crate::domain::deck::Deck;
use crate::domain::table::Table;
use crate::domain::tournament::Tournament;
use crate::domain::{HandId, PlayerId, SeatIndex, TableId, TournamentId};
use crate::engine::betting::BettingState;
use crate::engine::hand_history::HandHistory;
use crate::engine::pot::Pot;
use crate::engine::side_pots::SidePot;

/// Снэпшот HandEngine, который можно хранить во View.
/// Это «замороженная» раздача: всё, что нужно, чтобы восстановить HandEngine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandEngineSnapshot {
    pub table_id: TableId,
    pub hand_id: HandId,
    pub deck: Deck,
    pub betting: BettingState,
    pub pot: Pot,
    pub side_pots: Vec<SidePot>,
    pub contributions: HashMap<SeatIndex, Chips>,
    pub current_actor: Option<SeatIndex>,
    pub history: HandHistory,
}

impl HandEngineSnapshot {
    /// Упаковать живой HandEngine в снапшот для хранения on-chain.
    pub fn from_engine(engine: &crate::engine::game_loop::HandEngine) -> Self {
        Self {
            table_id: engine.table_id,
            hand_id: engine.hand_id,
            deck: engine.deck.clone(),
            betting: engine.betting.clone(),
            pot: engine.pot.clone(),
            side_pots: engine.side_pots.clone(),
            contributions: engine.contributions.clone(),
            current_actor: engine.current_actor,
            history: engine.history.clone(),
        }
    }

    /// Развернуть снапшот обратно в HandEngine (в памяти).
    pub fn into_engine(self) -> crate::engine::game_loop::HandEngine {
        crate::engine::game_loop::HandEngine {
            table_id: self.table_id,
            hand_id: self.hand_id,
            deck: self.deck,
            betting: self.betting,
            pot: self.pot,
            side_pots: self.side_pots,
            contributions: self.contributions,
            current_actor: self.current_actor,
            history: self.history,
        }
    }
}

/// Глобальное состояние покерного приложения на Linera.
///
/// Важное:
/// - НЕ вкладываем RegisterView внутрь MapView.
/// - Храним доменные структуры напрямую (Table, Tournament),
///   а для HandEngine используем HandEngineSnapshot.
#[derive(RootView)]
#[view(context = ViewStorageContext)]
pub struct PokerState {
    /// Все кэш- и турнирные столы.
    ///
    /// Ключ: TableId (u64 / alias),
    /// Значение: доменная структура Table (Serialize + Deserialize).
    #[view(map)]
    pub tables: MapView<TableId, Table>,

    /// Активные раздачи по каждому столу.
    ///
    /// Ключ: TableId,
    /// Значение: Option<HandEngineSnapshot> (None, если сейчас раздачи нет).
    #[view(map)]
    pub active_hands: MapView<TableId, Option<HandEngineSnapshot>>,

    /// Турниры (доменные структуры).
    ///
    /// Ключ: TournamentId,
    /// Значение: Tournament (из crate::domain::tournament).
    #[view(map)]
    pub tournaments: MapView<TournamentId, Tournament>,

    /// Сколько всего раздач сыграно (для статистики / мониторинга).
    #[view(register)]
    pub total_hands_played: RegisterView<u64>,

    /// Имена игроков для фронта: PlayerId -> отображаемое имя.
    #[view(map)]
    pub player_names: MapView<PlayerId, String>,
}
