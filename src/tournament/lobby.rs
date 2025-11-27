// src/tournament/lobby.rs

use std::collections::HashMap;

use crate::domain::{PlayerId, TournamentId};
use crate::domain::tournament::{Tournament, TournamentConfig, TournamentError};

/// Простое турнирное лобби:
/// - хранит турниры в памяти;
/// - выдаёт новые TournamentId;
/// - умеет создавать турниры;
/// - умеет регистрировать игроков в эти турниры.
pub struct TournamentLobby {
    tournaments: HashMap<TournamentId, Tournament>,
    next_tournament_id: TournamentId,
}

impl TournamentLobby {
    /// Пустое лобби, без турниров.
    pub fn new() -> Self {
        Self {
            tournaments: HashMap::new(),
            // id начинаем с 1, как и столы.
            next_tournament_id: 1,
        }
    }

    /// Создать новый турнир.
    ///
    /// На вход:
    /// - `config` — конфигурация турнира (включая name, max_players, re-entry и т.д.).
    ///
    /// Возвращает:
    /// - `TournamentId` созданного турнира.
    pub fn create_tournament(
        &mut self,
        owner: PlayerId,
        config: TournamentConfig,
    ) -> Result<TournamentId, TournamentError> {
        let id = self.next_tournament_id;
        self.next_tournament_id += 1;

        let tournament = Tournament::new(id, owner, config)?; // ← передаём owner и распаковываем Result

        self.tournaments.insert(id, tournament);
        Ok(id)
    }

    /// Получить турнир по id (только чтение).
    pub fn get(&self, id: TournamentId) -> Option<&Tournament> {
        self.tournaments.get(&id)
    }

    /// Получить турнир по id (для изменения).
    pub fn get_mut(&mut self, id: TournamentId) -> Option<&mut Tournament> {
        self.tournaments.get_mut(&id)
    }

    /// Вернуть все турниры (например, для отображения в фронте).
    pub fn all(&self) -> impl Iterator<Item = (&TournamentId, &Tournament)> {
        self.tournaments.iter()
    }

    /// Удобный метод для регистрации игрока в турнир.
    pub fn register_player(
        &mut self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), TournamentError> {
        let tournament = self
            .tournaments
            .get_mut(&tournament_id)
            .ok_or(TournamentError::TournamentNotFound { tournament_id })?;

        tournament.register_player(player_id)
    }
}
