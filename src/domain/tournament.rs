// src/domain/tournament.rs

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::chips::Chips;
use crate::domain::{PlayerId, TableId, TournamentId, SeatIndex};

/// Конфигурация турнира.
///
/// Это то, что в будущем ты будешь задавать в "создать турнир" (как на PokerNow / LePoker).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TournamentConfig {
    /// Название турнира (Daily 10k, Sunday Major и т.д.).
    pub name: String,
    /// Стартовый стек в фишках.
    pub starting_stack: Chips,
    /// Размер стола (6-max, 9-max и т.п.).
    pub table_size: u8,
    /// True, если турнир фриз-аут (одна жизнь).
    pub freezeout: bool,
    /// Разрешены ли ре-энтри (повторные входы после вылета).
    pub reentry_allowed: bool,
    /// Максимальное количество игроков (уникальных PlayerId),
    /// которые могут зарегистрироваться в турнир.
    pub max_players: u32,
    /// Максимальное количество ре-энтри на игрока.
    ///
    /// Интерпретация:
    /// - если `reentry_allowed = false`, поле игнорируется;
    /// - если `reentry_allowed = true`, то
    ///   максимальное число ЗАХОДОВ игрока = 1 (первый вход) + max_reentries_per_player.
    pub max_reentries_per_player: u32,
}

/// Статус турнира.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TournamentStatus {
    /// Идёт регистрация, можно регаться / отрегаться.
    Registering,
    /// Турнир запущен, регистрация закрыта (или late reg).
    Running,
    /// Турнир завершён.
    Finished,
}

/// Информация о регистрации конкретного игрока в турнире.
/// Здесь же храним турнирные поля: стек, busted, стол, место.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerRegistration {
    pub player_id: PlayerId,
    /// Сколько раз игрок уже заходил в турнир (включая первый вход).
    pub entries_used: u32,
    /// Выбыл ли игрок из турнира.
    pub is_busted: bool,
    /// За каким столом сидит (логический id стола турнира).
    pub table_id: Option<TableId>,
    /// Какой у него seat за столом (0-based).
    pub seat_index: Option<SeatIndex>,
    /// Текущий турнирный стек игрока.
    pub stack: Chips,
}

/// Ошибки, связанные с турниром.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TournamentError {
    /// Регистрация закрыта (турнир уже запущен или завершён).
    #[error("registration is closed for this tournament")]
    RegistrationClosed,

    /// В турнире больше нельзя регистрировать новых игроков (достигнут лимит мест).
    #[error("tournament is full")]
    TournamentFull,

    /// Игрок превысил лимит ре-эн́три.
    #[error("too many re-entries for player {player_id}")]
    TooManyReentries { player_id: PlayerId },

    /// Турнир не найден (используется на уровне лобби).
    #[error("tournament {tournament_id} not found")]
    TournamentNotFound { tournament_id: TournamentId },
}

/// Основная структура турнира.
///
/// Пока это чисто доменная модель, без ончейна и без привязки к конкретному движку столов.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tournament {
    /// Идентификатор турнира.
    pub id: TournamentId,
    /// Конфигурация.
    pub config: TournamentConfig,
    /// Текущий статус.
    pub status: TournamentStatus,
    /// Логические столы турнира (id столов турнира).
    pub tables: Vec<TableId>,
    /// Регистрации игроков: PlayerId -> PlayerRegistration.
    registrations: HashMap<PlayerId, PlayerRegistration>,
}

impl Tournament {
    /// Создать новый турнир в статусе `Registering`, без игроков и столов.
    pub fn new(id: TournamentId, config: TournamentConfig) -> Self {
        Self {
            id,
            config,
            status: TournamentStatus::Registering,
            tables: Vec::new(),
            registrations: HashMap::new(),
        }
    }

    /// Текущая максимальная разрешённая "глубина входа" для игрока.
    ///
    /// Если ре-энтри выключены, то это всегда 1.
    pub fn max_entries_per_player(&self) -> u32 {
        if !self.config.reentry_allowed {
            1
        } else {
            1 + self.config.max_reentries_per_player
        }
    }

    /// Сколько сейчас зарегистрировано уникальных игроков.
    pub fn current_player_count(&self) -> usize {
        self.registrations.len()
    }

    /// Есть ли ещё свободные места по `max_players`.
    pub fn has_free_seats(&self) -> bool {
        (self.current_player_count() as u32) < self.config.max_players
    }

    /// Получить регистрацию игрока, если есть.
    pub fn registration_for(&self, player_id: PlayerId) -> Option<&PlayerRegistration> {
        self.registrations.get(&player_id)
    }

    /// Мутируемая ссылка на регистрацию игрока.
    ///
    /// Используется, чтобы обновлять турнирные поля (stack, table_id, seat_index и т.п.)
    /// из движка столов / симуляции турнира.
    pub fn registration_for_mut(
        &mut self,
        player_id: PlayerId,
    ) -> Option<&mut PlayerRegistration> {
        self.registrations.get_mut(&player_id)
    }

    /// Итератор по всем регистрациям (для статистики, фронта и т.д.).
    pub fn registrations_iter(
        &self,
    ) -> impl Iterator<Item = (&PlayerId, &PlayerRegistration)> {
        self.registrations.iter()
    }

    /// Итератор по всем зарегистрированным игрокам (только PlayerId).
    pub fn players(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.registrations.keys().copied()
    }

    /// Зарегистрировать игрока в турнир.
    ///
    /// Логика:
    /// - если статус не `Registering` → `RegistrationClosed`;
    /// - если игрок регается впервые:
    ///     - проверяем лимит `max_players`;
    ///     - создаём запись с `entries_used = 1`;
    ///     - выставляем `stack = starting_stack`;
    /// - если игрок уже был (ре-энтри):
    ///     - проверяем, что `entries_used < max_entries_per_player`;
    ///     - увеличиваем `entries_used` на 1;
    ///     - сбрасываем `is_busted = false`, стек = `starting_stack`,
    ///       `table_id` и `seat_index` очистим, seating потом повесим заново.
    pub fn register_player(
        &mut self,
        player_id: PlayerId,
    ) -> Result<(), TournamentError> {
        // 1. Проверка статуса.
        if self.status != TournamentStatus::Registering {
            return Err(TournamentError::RegistrationClosed);
        }

        let max_entries = self.max_entries_per_player();

        match self.registrations.get_mut(&player_id) {
            // Игрок уже был в турнире — делаем ре-энтри.
            Some(reg) => {
                if reg.entries_used >= max_entries {
                    return Err(TournamentError::TooManyReentries { player_id });
                }
                reg.entries_used += 1;
                reg.is_busted = false;
                reg.stack = self.config.starting_stack;
                reg.table_id = None;
                reg.seat_index = None;
                Ok(())
            }
            // Первый вход игрока.
            None => {
                // Проверяем лимит мест.
                if !self.has_free_seats() {
                    return Err(TournamentError::TournamentFull);
                }

                let reg = PlayerRegistration {
                    player_id,
                    entries_used: 1,
                    is_busted: false,
                    table_id: None,
                    seat_index: None,
                    stack: self.config.starting_stack,
                };
                self.registrations.insert(player_id, reg);
                Ok(())
            }
        }
    }

    /// (Опционально) Отмена регистрации или снятие с турнира.
    ///
    /// В нашей оффчейн-симуляции турнирного движка мы используем это
    /// именно как "вылет из турнира": игрок полностью покидает список
    /// активных регистраций.
    pub fn unregister_player(&mut self, player_id: PlayerId) {
        self.registrations.remove(&player_id);
    }

    /// Количество активных (не вылетевших) игроков.
    ///
    /// Обрати внимание: в текущей оффчейн-симуляции мы при вылете
    /// полностью убираем регистрацию (`unregister_player`), поэтому
    /// `active_player_count` и `current_player_count` в реальной
    /// симуляции будут совпадать. Это поле остаётся на будущее,
    /// если захочешь хранить вылетевших в списке с `is_busted = true`.
    pub fn active_player_count(&self) -> usize {
        self.registrations.values().filter(|p| !p.is_busted).count()
    }

    /// Итерируемся по активным игрокам.
    pub fn active_registrations(&self) -> impl Iterator<Item = &PlayerRegistration> {
        self.registrations.values().filter(|p| !p.is_busted)
    }

    /// Вернуть список id столов турнира (логические столы турнира).
    pub fn table_ids(&self) -> &[TableId] {
        &self.tables
    }

    /// Итерируемся по активным игрокам конкретного стола.
    pub fn players_on_table(
        &self,
        table_id: TableId,
    ) -> impl Iterator<Item = &PlayerRegistration> {
        self.registrations
            .values()
            .filter(move |p| !p.is_busted && p.table_id == Some(table_id))
    }

    /// Полностью пересобрать рассадку игроков по столам.
    ///
    /// - очистит self.tables;
    /// - сбросит у всех игроков table_id/seat_index;
    /// - разложит активных игроков по столам по config.table_size;
    /// - присвоит столам id, начиная с `starting_table_id`.
    pub fn rebuild_seating_from_scratch(&mut self, starting_table_id: TableId) {
        // Сначала очищаем все старые привязки.
        self.tables.clear();
        for reg in self.registrations.values_mut() {
            reg.table_id = None;
            reg.seat_index = None;
        }

        let table_size = self.config.table_size as usize;
        if table_size == 0 {
            // Невалидная конфигурация, просто выходим.
            return;
        }

        // Берём всех живых игроков и сортируем по id,
        // чтобы seating был стабильным/детерминированным.
        let mut active_player_ids: Vec<PlayerId> = self
            .registrations
            .values()
            .filter(|p| !p.is_busted)
            .map(|p| p.player_id)
            .collect();

        active_player_ids.sort_unstable();

        let mut next_table_id = starting_table_id;
        let mut idx = 0;

        while idx < active_player_ids.len() {
            let end = usize::min(idx + table_size, active_player_ids.len());
            let chunk = &active_player_ids[idx..end];

            let table_id = next_table_id;
            self.tables.push(table_id);

            // Рассаживаем игроков по местам 0..(chunk.len()-1)
            for (seat, player_id) in chunk.iter().enumerate() {
                if let Some(reg) = self.registrations.get_mut(player_id) {
                    reg.table_id = Some(table_id);
                    reg.seat_index = Some(seat as SeatIndex);
                }
            }

            next_table_id += 1;
            idx = end;
        }
    }
}
