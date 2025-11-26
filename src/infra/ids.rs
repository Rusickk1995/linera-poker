use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::domain::{HandId, PlayerId, TableId, TournamentId};

/// Простая генерация ID на основе монотонных счётчиков.
/// Это удобно для локальных тестов, оффчейн-сервисов и т.д.
///
/// В Linera-смартконтракте ID чаще всего генерятся:
/// - из внешнего контекста (клиент сам передаёт),
/// - или из счётчиков в state (например, total_hands_played).
#[derive(Debug)]
pub struct IdGenerator {
    table_counter: AtomicU64,
    player_counter: AtomicU64,
    tournament_counter: AtomicU64,
    hand_counter: AtomicU64,
}

impl IdGenerator {
    /// Создать генератор с начальным значением 1 для всех сущностей.
    pub fn new() -> Self {
        Self {
            table_counter: AtomicU64::new(1),
            player_counter: AtomicU64::new(1),
            tournament_counter: AtomicU64::new(1),
            hand_counter: AtomicU64::new(1),
        }
    }

    #[inline]
    pub fn next_table_id(&self) -> TableId {
        self.table_counter.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn next_player_id(&self) -> PlayerId {
        self.player_counter.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn next_tournament_id(&self) -> TournamentId {
        self.tournament_counter.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn next_hand_id(&self) -> HandId {
        self.hand_counter.fetch_add(1, Ordering::Relaxed)
    }
}

/// Иногда удобно иметь "человекочитаемый" внешний ID,
/// но внутри всё равно использовать числовые.
/// На будущее – тип-обёртка над строкой.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ExternalId(pub String);
