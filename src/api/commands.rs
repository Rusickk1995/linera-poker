use serde::{Deserialize, Serialize};

use crate::domain::chips::Chips;
use crate::domain::{PlayerId, TableId, TournamentId};
use crate::domain::tournament::TournamentConfig;
use crate::engine::actions::PlayerAction;

/// Команда верхнего уровня.
///
/// Эти команды превращаются в операции (`PokerOperation`),
/// которые Linera экспонирует наружу в виде GraphQL mutations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    /// Создать новый стол (кэш или турнирный).
    CreateTable(CreateTableCommand),

    /// Операция над конкретным столом.
    TableCommand(TableCommand),

    /// Турнирные команды верхнего уровня.
    ///
    /// Через них фронт управляет турниром:
    /// - создаёт турнир;
    /// - регистрирует игроков;
    /// - запускает турнир;
    /// - переводит уровни блайндов;
    /// - завершает турнир.
    TournamentCommand(TournamentCommand),
}

/// Команда создания стола.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateTableCommand {
    /// Идентификатор нового стола.
    pub table_id: TableId,
    /// Имя стола (отображается во фронте / лобби).
    pub name: String,
    /// Максимальное количество мест (2–9, например).
    pub max_seats: u8,
    /// Блайнды / анте.
    pub small_blind: Chips,
    pub big_blind: Chips,
    pub ante: Chips,
    /// Тип анте: none / classic / big blind.
    ///
    /// На уровне API используем отдельный enum,
    /// в домене маппим в `domain::blinds::AnteType`.
    pub ante_type: AnteTypeApi,
}

/// Внешнее представление типа анте (API-слой).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnteTypeApi {
    None,
    Classic,
    BigBlind,
}

/// Команды, которые относятся к существующему столу.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TableCommand {
    /// Посадить игрока за стол.
    SeatPlayer(SeatPlayerCommand),

    /// Убрать игрока с места.
    UnseatPlayer(UnseatPlayerCommand),

    /// Изменить стек игрока (кэш-ин/кэш-аут).
    AdjustStack(AdjustStackCommand),

    /// Запустить новую раздачу (если сейчас нет активной).
    StartHand(StartHandCommand),

    /// Действие игрока в раздаче.
    PlayerAction(PlayerActionCommand),
}

/// Посадить игрока в конкретное место.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeatPlayerCommand {
    pub table_id: TableId,
    pub player_id: PlayerId,
    pub seat_index: u8,
    pub display_name: String,
    pub initial_stack: Chips,
}

/// Убрать игрока с места.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnseatPlayerCommand {
    pub table_id: TableId,
    pub seat_index: u8,
}

/// Добавить/убрать фишки игроку (кэш-игра).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdjustStackCommand {
    pub table_id: TableId,
    pub seat_index: u8,
    /// Может быть отрицательным — кэш-аут.
    pub delta: i64,
}

/// Запуск новой раздачи.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartHandCommand {
    pub table_id: TableId,
    /// Идентификатор раздачи (можно генерить в state).
    pub hand_id: u64,
}

/// Действие игрока в раздаче.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerActionCommand {
    pub table_id: TableId,
    pub action: PlayerAction,
}

/// Турнирные команды верхнего уровня.
///
/// Они работают поверх доменной логики Tournament / TournamentLobby / TournamentRuntime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TournamentCommand {
    /// Создать новый турнир с заданным конфигом.
    ///
    /// Конфиг включает:
    /// - имя турнира;
    /// - бай-ин / рейк;
    /// - начальный стек;
    /// - структуру уровней блайндов;
    /// - размер столов;
    /// - максимальное количество игроков и пр.
    CreateTournament(CreateTournamentCommand),

    /// Зарегистрировать игрока в турнир.
    ///
    /// Игрок попадает в список регистраций, но ещё не посажен за стол.
    RegisterPlayer(RegisterPlayerInTournamentCommand),

    /// Отменить регистрацию игрока (до старта турнира).
    UnregisterPlayer(UnregisterPlayerFromTournamentCommand),

    /// Старт турнира:
    /// - статус турнира → Running;
    /// - по всем зарегистрированным игрокам происходит рассадка по столам;
    /// - создаются/обновляются турнирные столы в состоянии.
    StartTournament(StartTournamentCommand),

    /// Принудительно перевести турнир на следующий уровень блайндов.
    ///
    /// Может вызываться:
    /// - по таймеру (off-chain, фронт дергает по расписанию);
    /// - вручную “директором турнира”.
    AdvanceLevel(AdvanceLevelCommand),

    /// Завершить турнир, когда остался один победитель.
    ///
    /// В доменной логике турнир переходит в статус Finished,
    /// можно отображать призы/результаты.
    CloseTournament(CloseTournamentCommand),
}

/// Команда на создание турнира.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateTournamentCommand {
    /// Полная конфигурация турнира (см. `domain::tournament::TournamentConfig`).
    pub config: TournamentConfig,
}

/// Зарегистрировать игрока в турнир.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterPlayerInTournamentCommand {
    pub tournament_id: TournamentId,
    pub player_id: PlayerId,
    /// Отображаемое имя игрока в лобби/на столах.
    pub display_name: String,
}

/// Отменить регистрацию игрока (пока турнир не начался).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnregisterPlayerFromTournamentCommand {
    pub tournament_id: TournamentId,
    pub player_id: PlayerId,
}

/// Старт турнира: создать столы, рассадить игроков, включить первый уровень блайндов.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartTournamentCommand {
    pub tournament_id: TournamentId,
}

/// Перейти на следующий уровень блайндов.
///
/// Конкретный уровень определяется доменной логикой турнира
/// (например, по расписанию уровней в `TournamentConfig`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdvanceLevelCommand {
    pub tournament_id: TournamentId,
}

/// Завершить турнир.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloseTournamentCommand {
    pub tournament_id: TournamentId,
}
