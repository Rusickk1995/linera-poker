use std::collections::HashMap;

use crate::domain::table::Table;
use crate::domain::tournament::Tournament;
use crate::domain::{TableId, TournamentId};
use crate::state::HandEngineSnapshot;

/// Абстракция хранилища для покера.
///
/// В Linera-режиме вместо этого используется `PokerState` и Views,
/// но эта абстракция удобна:
/// - для юнит- и интеграционных тестов движка,
/// - для оффчейн-сервисов (например, lobby-сервер).
pub trait PokerStorage {
    /// Загрузить стол.
    fn load_table(&self, id: TableId) -> Option<Table>;

    /// Сохранить стол.
    fn save_table(&mut self, table: &Table);

    /// Загрузить активную раздачу для стола (если она есть).
    fn load_active_hand(&self, table_id: TableId) -> Option<HandEngineSnapshot>;

    /// Сохранить / очистить активную раздачу.
    fn save_active_hand(&mut self, table_id: TableId, snapshot: Option<HandEngineSnapshot>);

    /// Загрузить турнир.
    fn load_tournament(&self, id: TournamentId) -> Option<Tournament>;

    /// Сохранить турнир.
    fn save_tournament(&mut self, tournament: &Tournament);
}

/// Простая in-memory реализация для тестов и локального запуска.
#[derive(Debug, Default)]
pub struct InMemoryPokerStorage {
    tables: HashMap<TableId, Table>,
    active_hands: HashMap<TableId, HandEngineSnapshot>,
    tournaments: HashMap<TournamentId, Tournament>,
}

impl InMemoryPokerStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PokerStorage for InMemoryPokerStorage {
    fn load_table(&self, id: TableId) -> Option<Table> {
        self.tables.get(&id).cloned()
    }

    fn save_table(&mut self, table: &Table) {
        self.tables.insert(table.id, table.clone());
    }

    fn load_active_hand(&self, table_id: TableId) -> Option<HandEngineSnapshot> {
        self.active_hands.get(&table_id).cloned()
    }

    fn save_active_hand(&mut self, table_id: TableId, snapshot: Option<HandEngineSnapshot>) {
        if let Some(s) = snapshot {
            self.active_hands.insert(table_id, s);
        } else {
            self.active_hands.remove(&table_id);
        }
    }

    fn load_tournament(&self, id: TournamentId) -> Option<Tournament> {
        self.tournaments.get(&id).cloned()
    }

    fn save_tournament(&mut self, tournament: &Tournament) {
        self.tournaments.insert(tournament.id, tournament.clone());
    }
}
