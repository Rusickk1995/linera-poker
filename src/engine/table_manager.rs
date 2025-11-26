// src/engine/table_manager.rs

use std::collections::HashMap;

use crate::domain::{HandId, SeatIndex, TableId};
use crate::domain::table::Table;
use crate::engine::{self, HandEngine, HandStatus, PlayerAction, EngineError};
use crate::engine::RandomSource;

/// Ошибки уровня менеджера столов (над движком одной раздачи).
#[derive(Debug)]
pub enum ManagerError {
    /// Стол с таким ID не найден.
    TableNotFound(TableId),

    /// Для стола ещё не запущена раздача (нет HandEngine).
    NoActiveHand(TableId),

    /// Проброшенная ошибка из движка (EngineError).
    Engine(EngineError),
}

impl From<EngineError> for ManagerError {
    fn from(e: EngineError) -> Self {
        ManagerError::Engine(e)
    }
}

/// Внутренний объект: один стол + опционально активный движок раздачи.
struct ManagedTable {
    table: Table,
    engine: Option<HandEngine>,
}

impl ManagedTable {
    fn new(table: Table) -> Self {
        Self {
            table,
            engine: None,
        }
    }
}

/// Менеджер столов:
/// - хранит несколько столов по TableId;
/// - для каждого стола может быть активный HandEngine (текущая раздача);
/// - даёт методы start_hand/apply_action поверх engine::start_hand / engine::apply_action.
pub struct TableManager {
    tables: HashMap<TableId, ManagedTable>,
}

impl TableManager {
    /// Создать пустой менеджер.
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    /// Добавить стол под его TableId.
    ///
    /// Если стол с таким id уже был — заменяем его.
    /// (Можно поменять на вставку с проверкой, если нужно.)
    pub fn add_table(&mut self, table: Table) {
        let id = table.id;
        self.tables.insert(id, ManagedTable::new(table));
    }

    /// Есть ли стол с таким id.
    pub fn has_table(&self, table_id: TableId) -> bool {
        self.tables.contains_key(&table_id)
    }

    /// Получить ссылку на стол (read-only).
    pub fn table(&self, table_id: TableId) -> Option<&Table> {
        self.tables.get(&table_id).map(|mt| &mt.table)
    }

    /// Получить ссылку на стол (mutable).
    ///
    /// Важно: пока у тебя нет активного фронта/сети, этого достаточно.
    /// Для жёсткой инварианты можно запретить мутировать стол во время активной раздачи,
    /// но пока оставим гибко.
    pub fn table_mut(&mut self, table_id: TableId) -> Option<&mut Table> {
        self.tables.get_mut(&table_id).map(|mt| &mut mt.table)
    }

    /// Есть ли активная раздача на столе.
    pub fn has_active_hand(&self, table_id: TableId) -> bool {
        self.tables
            .get(&table_id)
            .and_then(|mt| mt.engine.as_ref())
            .is_some()
    }

    /// Получить ссылку на HandEngine (read-only) для стола.
    pub fn hand_engine(&self, table_id: TableId) -> Option<&HandEngine> {
        self.tables
            .get(&table_id)
            .and_then(|mt| mt.engine.as_ref())
    }

    /// Получить ссылку на HandEngine (mutable) для стола.
    pub fn hand_engine_mut(&mut self, table_id: TableId) -> Option<&mut HandEngine> {
        self.tables
            .get_mut(&table_id)
            .and_then(|mt| mt.engine.as_mut())
    }

    /// Текущий актёр на столе (если есть активная раздача).
    pub fn current_actor_seat(&self, table_id: TableId) -> Option<SeatIndex> {
        self.hand_engine(table_id)
            .and_then(|e| e.current_actor)
    }

    /// Запустить новую раздачу на конкретном столе.
    ///
    /// ВАЖНО: hand_id ты по-прежнему генерируешь через свой IdGenerator снаружи.
    /// Менеджер только оборачивает вызов engine::start_hand и хранит HandEngine.
    pub fn start_hand<R: RandomSource>(
        &mut self,
        table_id: TableId,
        rng: &mut R,
        hand_id: HandId,
    ) -> Result<(), ManagerError> {
        let mt = self
            .tables
            .get_mut(&table_id)
            .ok_or(ManagerError::TableNotFound(table_id))?;

        // Вызов движка одной раздачи.
        let engine = engine::start_hand(&mut mt.table, rng, hand_id)?;
        mt.engine = Some(engine);

        Ok(())
    }

    /// Применить действие игрока на конкретном столе.
    ///
    /// Внутри просто вызывает engine::apply_action для (Table, HandEngine).
    pub fn apply_action(
        &mut self,
        table_id: TableId,
        action: PlayerAction,
    ) -> Result<HandStatus, ManagerError> {
        let mt = self
            .tables
            .get_mut(&table_id)
            .ok_or(ManagerError::TableNotFound(table_id))?;

        let engine = mt
            .engine
            .as_mut()
            .ok_or(ManagerError::NoActiveHand(table_id))?;

        let status = engine::apply_action(&mut mt.table, engine, action)?;

        // Если раздача завершилась, по желанию можно:
        // - оставить snapshot HandEngine (чтобы читать историю позже),
        // - либо очищать mt.engine = None.
        //
        // Пока НИЧЕГО НЕ МЕНЯЕМ, чтобы не ломать привычную логику:
        // HandEngine остаётся внутри, и ты можешь дальше читать history/table state.

        Ok(status)
    }
}
