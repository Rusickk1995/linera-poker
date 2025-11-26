//! Покерный движок: ставки, переход улиц, сайд-поты, шоудаун.
//!
//! Высокоуровневый объект: `HandEngine`
//! Основные операции:
//!   - `start_hand` – запустить новую раздачу
//!   - `apply_action` – применить действие игрока
//!   - `advance_if_needed` – авто-переход улиц/завершение раздачи

pub mod actions;
pub mod betting;
pub mod errors;
pub mod game_loop;
pub mod hand_history;
pub mod positions;
pub mod pot;
pub mod side_pots;
pub mod validation;
pub mod table_manager;


pub use actions::{PlayerAction, PlayerActionKind};
pub use errors::EngineError;
pub use game_loop::{advance_if_needed, apply_action, start_hand, HandEngine, HandStatus};
pub use hand_history::{HandEvent, HandEventKind, HandHistory};
pub use pot::Pot;
pub use side_pots::SidePot;

/// RNG интерфейс для engine.
/// Реализацию дадим позже в infra (например, обёртка над `rand`).
pub trait RandomSource {
    fn shuffle<T>(&mut self, slice: &mut [T]);
}

pub use table_manager::{TableManager, ManagerError};
