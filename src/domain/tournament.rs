// src/domain/tournament.rs

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::blinds::{BlindLevel, BlindStructure};
use crate::domain::chips::Chips;
use crate::domain::{PlayerId, SeatIndex, TableId, TournamentId};

/// Расписание турнира.
///
/// Все времена – Unix timestamp в секундах (UTC).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TournamentScheduleConfig {
    /// Запланированное время старта турнира.
    ///
    /// Если 0 – значит "старт по кнопке", без жёсткого расписания.
    pub scheduled_start_ts: u64,

    /// Можно ли стартовать раньше, чем scheduled_start_ts,
    /// если набран минимум игроков.
    pub allow_start_earlier: bool,

    /// Каждые сколько минут делаем перерыв.
    ///
    /// Пример: 60 = перерыв раз в час.
    pub break_every_minutes: u32,

    /// Длительность перерыва в минутах.
    ///
    /// Пример: 5 = перерыв 5 минут.
    pub break_duration_minutes: u32,
}

impl TournamentScheduleConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.break_every_minutes == 0 {
            return Err("TournamentScheduleConfig: break_every_minutes = 0".into());
        }
        if self.break_duration_minutes == 0 {
            return Err("TournamentScheduleConfig: break_duration_minutes = 0".into());
        }
        Ok(())
    }

    /// Удобный пресет: перерыв раз в час на 5 минут.
    pub fn hourly_with_five_min_break() -> Self {
        Self {
            scheduled_start_ts: 0,
            allow_start_earlier: true,
            break_every_minutes: 60,
            break_duration_minutes: 5,
        }
    }
}

/// Настройки балансировки столов.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableBalancingConfig {
    /// Включена ли вообще балансировка.
    pub enabled: bool,
    /// Максимально допустимая разница по количеству игроков
    /// между самым полным и самым пустым столом.
    /// Обычно 1 или 2.
    pub max_seat_diff: u8,
}

impl TableBalancingConfig {
    pub fn validate(&self, table_size: u8) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        if self.max_seat_diff == 0 {
            return Err("TableBalancingConfig: max_seat_diff = 0".into());
        }
        if self.max_seat_diff as u8 >= table_size {
            return Err("TableBalancingConfig: max_seat_diff >= table_size".into());
        }
        Ok(())
    }

    pub fn default_with_diff_one() -> Self {
        Self {
            enabled: true,
            max_seat_diff: 1,
        }
    }
}

/// Конфигурация турнира.
/// Всё, что приходит при создании турнира через фронт.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TournamentConfig {
    /// Название турнира.
    pub name: String,

    /// Краткое описание.
    pub description: Option<String>,

    /// Размер стартового стека.
    pub starting_stack: Chips,

    /// Максимальное количество игроков в турнире (cap).
    pub max_players: u32,

    /// Минимальное количество игроков для старта (например, 2 или 9).
    pub min_players_to_start: u32,

    /// Размер стола (2–9).
    pub table_size: u8,

    /// Freezeout = без реэнтри.
    pub freezeout: bool,

    /// Разрешены ли реэнтри (повторная регистрация после вылета).
    pub reentry_allowed: bool,

    /// Максимальное количество входов на одного игрока.
    ///
    /// Для классического freezeout = 1.
    pub max_entries_per_player: u32,

    /// До какого уровня (включительно) разрешена поздняя регистрация (late reg).
    /// 0 = без late reg.
    pub late_reg_level: u32,

    /// Структура блайндов/анте и их длительности.
    pub blind_structure: BlindStructure,

    /// Автоаппрув регистрации (true) или ручной аппрув (false).
    pub auto_approve: bool,

    /// Расписание (старт + перерывы).
    pub schedule: TournamentScheduleConfig,

    /// Настройки балансировки столов.
    pub balancing: TableBalancingConfig,
}

impl TournamentConfig {
    /// Жёсткая валидация конфига турнира.
    pub fn validate_full(&self) -> Result<(), TournamentError> {
        if self.name.trim().is_empty() {
            return Err(TournamentError::InvalidConfig(
                "TournamentConfig: name is empty".into(),
            ));
        }

        if self.starting_stack.0 == 0 {
            return Err(TournamentError::InvalidConfig(
                "TournamentConfig: starting_stack = 0".into(),
            ));
        }

        if self.max_players == 0 {
            return Err(TournamentError::InvalidConfig(
                "TournamentConfig: max_players = 0".into(),
            ));
        }

        if self.min_players_to_start == 0 {
            return Err(TournamentError::InvalidConfig(
                "TournamentConfig: min_players_to_start = 0".into(),
            ));
        }

        if self.min_players_to_start > self.max_players {
            return Err(TournamentError::InvalidConfig(
                "TournamentConfig: min_players_to_start > max_players".into(),
            ));
        }

        if self.table_size < 2 || self.table_size > 9 {
            return Err(TournamentError::InvalidConfig(
                "TournamentConfig: table_size must be in [2, 9]".into(),
            ));
        }

        if self.reentry_allowed && self.max_entries_per_player < 2 {
            return Err(TournamentError::InvalidConfig(
                "TournamentConfig: reentry_allowed but max_entries_per_player < 2".into(),
            ));
        }

        if !self.reentry_allowed && self.max_entries_per_player != 1 {
            return Err(TournamentError::InvalidConfig(
                "TournamentConfig: freezeout must have max_entries_per_player = 1".into(),
            ));
        }

        self.blind_structure
            .validate()
            .map_err(TournamentError::InvalidConfig)?;

        if self.late_reg_level > 0
            && self
                .blind_structure
                .level_by_number(self.late_reg_level)
                .is_none()
        {
            return Err(TournamentError::InvalidConfig(format!(
                "TournamentConfig: late_reg_level {} is out of bounds",
                self.late_reg_level
            )));
        }

        self.schedule
            .validate()
            .map_err(TournamentError::InvalidConfig)?;

        self.balancing
            .validate(self.table_size)
            .map_err(TournamentError::InvalidConfig)?;

        Ok(())
    }
}

/// Статус турнира.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TournamentStatus {
    Registering,
    Running,
    OnBreak,
    Finished,
}

/// Игрок в турнире (регистрация).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerRegistration {
    pub player_id: PlayerId,
    /// Текущий стек игрока в турнире (для старта/пересадок).
    pub total_chips: Chips,
    /// Вылетел ли игрок.
    pub is_busted: bool,
    /// На каком столе он сейчас сидит (если сидит).
    pub table_id: Option<TableId>,
    /// На каком месте за столом.
    pub seat_index: Option<SeatIndex>,
    /// Итоговое место в турнире (1 = победитель, N = первый вылет).
    pub finishing_place: Option<u32>,
}

pub type TournamentPlayer = PlayerRegistration;

/// Описание перестановки игрока при ребалансе столов (внутри Tournament).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RebalanceMove {
    pub player_id: PlayerId,
    pub from_table: TableId,
    pub to_table: TableId,
}

/// Событие, которое произошло при тиковом обновлении по времени.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TournamentTimeEvent {
    None,
    LevelAdvanced { from: u32, to: u32, new_blinds: BlindLevel },
    BreakStarted,
    BreakEnded,
}

/// Основной объект турнира.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tournament {
    pub id: TournamentId,
    pub owner: PlayerId,
    pub config: TournamentConfig,
    pub status: TournamentStatus,
    pub registrations: HashMap<PlayerId, PlayerRegistration>,
    pub current_level: u32,

    /// Фактическое время старта турнира (Unix timestamp).
    pub started_at_ts: Option<u64>,

    /// Время начала текущего уровня (Unix timestamp).
    pub level_started_at_ts: Option<u64>,

    /// Время начала текущего перерыва (если статус OnBreak).
    pub break_started_at_ts: Option<u64>,

    /// Общее количество участников на момент старта турнира.
    ///
    /// Нужно для детерминированного проставления мест:
    /// первый вылетит → место = total_entries,
    /// победитель → место = 1.
    pub total_entries: u32,

    /// Сколько игроков уже вылетело (для подсчёта мест).
    pub finished_count: u32,

    /// Победитель турнира (если уже известен).
    pub winner_id: Option<PlayerId>,
}

impl Tournament {
    pub fn new(
        id: TournamentId,
        owner: PlayerId,
        config: TournamentConfig,
    ) -> Result<Self, TournamentError> {
        config.validate_full()?;

        Ok(Self {
            id,
            owner,
            config,
            status: TournamentStatus::Registering,
            registrations: HashMap::new(),
            current_level: 1,
            started_at_ts: None,
            level_started_at_ts: None,
            break_started_at_ts: None,
            total_entries: 0,
            finished_count: 0,
            winner_id: None,
        })
    }

    pub fn current_blind_level(&self) -> &BlindLevel {
        self.config
            .blind_structure
            .level_by_number(self.current_level)
            .expect("Tournament.current_level must be valid")
    }

    /// Можно ли стартовать турнир в момент `now_ts`.
    pub fn can_start_now(&self, now_ts: u64) -> bool {
        if self.status != TournamentStatus::Registering {
            return false;
        }

        let players_count = self
            .registrations
            .values()
            .filter(|r| !r.is_busted)
            .count() as u32;

        if players_count < self.config.min_players_to_start {
            return false;
        }

        if self.config.schedule.scheduled_start_ts == 0 {
            // Старт "по кнопке" – расписание не ограничивает.
            return true;
        }

        if now_ts >= self.config.schedule.scheduled_start_ts {
            // Достигли планового времени старта.
            return true;
        }

        // Ранний старт – только если allow_start_earlier.
        self.config.schedule.allow_start_earlier
    }

    /// Помечает турнир как запущенный.
    pub fn start(&mut self, now_ts: u64) -> Result<(), TournamentError> {
        if !self.can_start_now(now_ts) {
            return Err(TournamentError::InvalidStatusForStart {
                status: self.status,
            });
        }

        self.status = TournamentStatus::Running;
        self.started_at_ts = Some(now_ts);
        self.level_started_at_ts = Some(now_ts);
        self.break_started_at_ts = None;
        self.current_level = 1;

        // Фиксируем количество участников на момент старта,
        // чтобы потом корректно выдавать места.
        self.total_entries = self.active_player_count() as u32;
        self.finished_count = 0;
        self.winner_id = None;

        Ok(())
    }

    /// Регистрируем игрока (пока турнир в статусе Registering).
    pub fn register_player(
        &mut self,
        player_id: PlayerId,
    ) -> Result<(), TournamentError> {
        if self.status != TournamentStatus::Registering {
            return Err(TournamentError::InvalidStatus {
                expected: TournamentStatus::Registering,
                found: self.status,
            });
        }

        if self.registrations.len() as u32 >= self.config.max_players {
            return Err(TournamentError::TournamentFull {
                tournament_id: self.id,
            });
        }

        if self.registrations.contains_key(&player_id) {
            return Err(TournamentError::AlreadyRegistered {
                player_id,
                tournament_id: self.id,
            });
        }

        let reg = PlayerRegistration {
            player_id,
            total_chips: self.config.starting_stack,
            is_busted: false,
            table_id: None,
            seat_index: None,
            finishing_place: None,
        };

        self.registrations.insert(player_id, reg);
        Ok(())
    }

    /// Активные (не вылетевшие) игроки.
    pub fn active_players(&self) -> impl Iterator<Item = &PlayerRegistration> {
        self.registrations.values().filter(|r| !r.is_busted)
    }

    /// Количество активных игроков.
    pub fn active_player_count(&self) -> usize {
        self.active_players().count()
    }

    /// Проверка, завершён ли турнир.
    ///
    /// Считаем завершённым, если статус Finished.
    pub fn is_finished(&self) -> bool {
        self.status == TournamentStatus::Finished
    }

    /// Пометить игрока как выбывшего (BUST).
    ///
    /// Важно:
    ///   - вызывать из движка (`engine/game_loop.rs`),
    ///     когда стек игрока стал 0;
    ///   - метод сам назначит место и обновит состояние турнира;
    ///   - если после вылета останется 1 активный игрок –
    ///     турнир автоматически завершится, победитель будет сохранён.
    ///
    /// Возвращает:
    ///   - Ok(finishing_place) – место, которое получил игрок;
    ///   - Err(...) – если нельзя пометить вылет.
    pub fn mark_player_busted(
        &mut self,
        player_id: PlayerId,
    ) -> Result<u32, TournamentError> {
        if self.status != TournamentStatus::Running {
            return Err(TournamentError::InvalidStatus {
                expected: TournamentStatus::Running,
                found: self.status,
            });
        }

        // Нельзя выбивать последнего игрока – защита от некорректных вызовов.
        if self.active_player_count() <= 1 {
            return Err(TournamentError::CannotBustLastPlayer {
                tournament_id: self.id,
            });
        }

        // ВАЖНО: это делаем ДО mutable borrow `reg`,
        // чтобы не конфликтовать с borrow checker.
        if self.total_entries == 0 {
            self.total_entries = self.active_player_count() as u32;
        }

        // Теперь берём mutable-ссылку на регистрацию игрока.
        let reg = self
            .registrations
            .get_mut(&player_id)
            .ok_or(TournamentError::NotRegistered {
                player_id,
                tournament_id: self.id,
            })?;

        if reg.is_busted {
            return Err(TournamentError::AlreadyBusted {
                player_id,
                tournament_id: self.id,
            });
        }

        // finishing_place = общее число участников - сколько уже вылетело.
        let finishing_place = self.total_entries.saturating_sub(self.finished_count);

        reg.is_busted = true;
        reg.finishing_place = Some(finishing_place);
        reg.table_id = None;
        reg.seat_index = None;

        self.finished_count = self.finished_count.saturating_add(1);

        // После вылета проверяем, не остался ли один игрок.
        self.check_and_finish_if_needed();

        Ok(finishing_place)
    }


    /// Тиковое обновление по времени:
    ///
    ///   - обновляет уровень блайндов, если прошло достаточно минут;
    ///   - включает/выключает перерыв по расписанию;
    ///   - возвращает, что произошло (`TournamentTimeEvent`).
    pub fn apply_time_tick(&mut self, now_ts: u64) -> TournamentTimeEvent {
        // В регистрационной или финальной фазе по времени ничего не делаем.
        if matches!(
            self.status,
            TournamentStatus::Finished | TournamentStatus::Registering
        ) {
            return TournamentTimeEvent::None;
        }

        let started_at = match self.started_at_ts {
            Some(ts) => ts,
            None => return TournamentTimeEvent::None,
        };

        let schedule = &self.config.schedule;
        let total_elapsed_secs = now_ts.saturating_sub(started_at);
        let total_elapsed_minutes = (total_elapsed_secs / 60) as u32;

        // Длина полного цикла "игра + перерыв".
        let cycle_minutes =
            schedule.break_every_minutes + schedule.break_duration_minutes;

        let cycle_pos = total_elapsed_minutes % cycle_minutes;

        match self.status {
            TournamentStatus::Running => {
                // Если мы в рабочем режиме и вошли в зону перерыва – стартуем перерыв.
                if cycle_pos >= schedule.break_every_minutes {
                    self.status = TournamentStatus::OnBreak;
                    self.break_started_at_ts = Some(now_ts);
                    return TournamentTimeEvent::BreakStarted;
                }
            }
            TournamentStatus::OnBreak => {
                // Если перерыв закончился – выходим из перерыва.
                if cycle_pos < schedule.break_every_minutes {
                    self.status = TournamentStatus::Running;
                    self.break_started_at_ts = None;

                    // При выходе с перерыва можно пересчитать уровень блайндов.
                    let ev = self.update_level_for_time(now_ts);
                    return if matches!(ev, TournamentTimeEvent::None) {
                        TournamentTimeEvent::BreakEnded
                    } else {
                        ev
                    };
                } else {
                    // Всё ещё на перерыве, ничего не меняем.
                    return TournamentTimeEvent::None;
                }
            }
            TournamentStatus::Finished | TournamentStatus::Registering => {
                return TournamentTimeEvent::None;
            }
        }

        // Если не было перерыва/выхода из перерыва – просто обновляем уровень блайндов.
        self.update_level_for_time(now_ts)
    }

    /// Внутренняя функция: обновить current_level, если по времени положено.
    fn update_level_for_time(
        &mut self,
        now_ts: u64,
    ) -> TournamentTimeEvent {
        let started_at = match self.started_at_ts {
            Some(ts) => ts,
            None => return TournamentTimeEvent::None,
        };

        let total_elapsed_minutes = ((now_ts.saturating_sub(started_at)) / 60) as u32;
        let target_level = self
            .config
            .blind_structure
            .level_for_elapsed_minutes(total_elapsed_minutes)
            .level;

        if target_level > self.current_level {
            let from = self.current_level;
            self.current_level = target_level;
            self.level_started_at_ts = Some(now_ts);
            let new_blinds = self.current_blind_level().clone();
            TournamentTimeEvent::LevelAdvanced {
                from,
                to: target_level,
                new_blinds,
            }
        } else {
            TournamentTimeEvent::None
        }
    }

    /// Рассадка игроков по столам при старте турнира (или полном пересборе).
    ///
    /// Используется при начале турнира или при полном пересчёте рассадки.
    /// Возвращает список:
    ///   (table_id, [player_id, ...])
    pub fn seat_players_evenly(
        &mut self,
        table_size: u8,
        mut next_table_id: TableId,
    ) -> Vec<(TableId, Vec<PlayerId>)> {
        let mut active: Vec<PlayerId> = self
            .registrations
            .values()
            .filter(|reg| !reg.is_busted)
            .map(|reg| reg.player_id)
            .collect();

        active.sort_unstable();

        let mut result = Vec::new();
        let ts = table_size.max(2) as usize;
        if active.is_empty() {
            return result;
        }

        let mut idx = 0usize;
        while idx < active.len() {
            let end = (idx + ts).min(active.len());
            let chunk = &active[idx..end];

            let table_id = next_table_id;
            let mut seated_ids = Vec::with_capacity(chunk.len());

            for (seat, player_id) in chunk.iter().enumerate() {
                if let Some(reg) = self.registrations.get_mut(player_id) {
                    reg.table_id = Some(table_id);
                    reg.seat_index = Some(seat as SeatIndex);
                }
                seated_ids.push(*player_id);
            }

            result.push((table_id, seated_ids));
            next_table_id += 1;
            idx = end;
        }

        result
    }

    /// Посчитать список перестановок игроков для ребаланса столов.
    ///
    /// Алгоритм:
    /// - считаем количество активных игроков на каждом столе;
    /// - пока разница между max и min > max_seat_diff:
    ///     берём одного игрока с самого полного стола и двигаем на самый пустой;
    /// - seat_index при этом обнуляем (потом пересядет движок стола).
    pub fn compute_rebalance_moves(&self) -> Vec<RebalanceMove> {
        if !self.config.balancing.enabled {
            return Vec::new();
        }

        // Собираем карты: table_id -> Vec<PlayerId>
        let mut table_map: HashMap<TableId, Vec<PlayerId>> = HashMap::new();

        for reg in self.active_players() {
            if let Some(tid) = reg.table_id {
                table_map.entry(tid).or_default().push(reg.player_id);
            }
        }

        if table_map.len() <= 1 {
            return Vec::new();
        }

        let mut moves = Vec::new();

        loop {
            // Находим столы с минимальным и максимальным количеством игроков.
            let mut min_tid = None;
            let mut max_tid = None;
            let mut min_count = u32::MAX;
            let mut max_count = 0u32;

            for (tid, players) in &table_map {
                let c = players.len() as u32;
                if c < min_count {
                    min_count = c;
                    min_tid = Some(*tid);
                }
                if c > max_count {
                    max_count = c;
                    max_tid = Some(*tid);
                }
            }

            if min_tid.is_none() || max_tid.is_none() {
                break;
            }

            let min_tid = min_tid.unwrap();
            let max_tid = max_tid.unwrap();

            if max_count - min_count <= self.config.balancing.max_seat_diff as u32 {
                break;
            }

            // Берём последнего игрока с самого полного стола.
            let from_vec = table_map.get_mut(&max_tid).unwrap();
            if from_vec.is_empty() {
                break;
            }
            let player_id = from_vec.pop().unwrap();

            let to_vec = table_map.get_mut(&min_tid).unwrap();
            to_vec.push(player_id);

            moves.push(RebalanceMove {
                player_id,
                from_table: max_tid,
                to_table: min_tid,
            });
        }

        moves
    }

    /// Применить список перестановок к Tournament (обновляет table_id/seat_index).
    pub fn apply_rebalance_moves(&mut self, moves: &[RebalanceMove]) {
        for m in moves {
            if let Some(reg) = self.registrations.get_mut(&m.player_id) {
                reg.table_id = Some(m.to_table);
                reg.seat_index = None;
            }
        }
    }

    /// Внутренняя логика: если остался один активный игрок – завершить турнир.
    ///
    /// - Если активных 0 → статус Finished, winner_id = None;
    /// - Если активный один → статус Finished, winner_id = Some(player),
    ///   ему ставим место 1 (если ещё не стоит).
    fn check_and_finish_if_needed(&mut self) {
        if self.status == TournamentStatus::Finished {
            return;
        }

        let mut active_ids: Vec<PlayerId> = self
            .active_players()
            .map(|r| r.player_id)
            .collect();

        let count = active_ids.len();

        if count == 0 {
            self.status = TournamentStatus::Finished;
            self.winner_id = None;
            return;
        }

        if count == 1 {
            active_ids.sort_unstable();
            let winner = active_ids[0];

            self.status = TournamentStatus::Finished;
            self.winner_id = Some(winner);

            // Если по какой-то причине место победителю ещё не проставилось –
            // ставим 1.
            if let Some(reg) = self.registrations.get_mut(&winner) {
                if reg.finishing_place.is_none() {
                    reg.finishing_place = Some(1);
                }
            }
        }
    }
}

/// Ошибки, которые могут возникать при работе с турниром.
#[derive(Debug, Error, Clone)]
pub enum TournamentError {
    #[error("Tournament not found: id={tournament_id}")]
    TournamentNotFound { tournament_id: TournamentId },

    #[error("Tournament is full: id={tournament_id}")]
    TournamentFull { tournament_id: TournamentId },

    #[error("Player {player_id} is already registered in tournament {tournament_id}")]
    AlreadyRegistered {
        player_id: PlayerId,
        tournament_id: TournamentId,
    },

    #[error("Player {player_id} is not registered in tournament {tournament_id}")]
    NotRegistered {
        player_id: PlayerId,
        tournament_id: TournamentId,
    },

    #[error("Player {player_id} is already busted in tournament {tournament_id}")]
    AlreadyBusted {
        player_id: PlayerId,
        tournament_id: TournamentId,
    },

    #[error("Cannot bust last remaining player in tournament {tournament_id}")]
    CannotBustLastPlayer { tournament_id: TournamentId },

    #[error("Invalid tournament status, expected {expected:?}, found {found:?}")]
    InvalidStatus {
        expected: TournamentStatus,
        found: TournamentStatus,
    },

    #[error("Invalid tournament status for start: {status:?}")]
    InvalidStatusForStart { status: TournamentStatus },

    #[error("Invalid tournament config: {0}")]
    InvalidConfig(String),
}
