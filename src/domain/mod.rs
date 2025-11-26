//! Доменная модель покера: карты, игроки, столы, турниры, блайнды и т.д.

pub mod blinds;
pub mod card;
pub mod chips;
pub mod deck;
pub mod hand;
pub mod player;
pub mod table;
pub mod tournament;

// Базовые идентификаторы (потом можно вынести в отдельный модуль ids/infra)
pub type PlayerId = u64;
pub type TableId = u64;
pub type TournamentId = u64;
pub type HandId = u64;

// Удобные реэкспорты, чтобы в других модулях писать crate::domain::Card и т.п.
pub use blinds::*;
pub use card::*;
pub use chips::*;
pub use deck::*;
pub use hand::*;
pub use player::*;
pub use table::*;
pub use tournament::*;
