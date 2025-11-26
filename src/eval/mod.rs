//! Модуль оценки силы покерных рук (Texas Hold'em).
//!
//! Основная функция:
//!   `evaluate_best_hand(hole, board) -> HandRank`

pub mod evaluator;
pub mod hand_rank;
pub mod lookup_tables;

pub use evaluator::evaluate_best_hand;
pub use hand_rank::{describe_hand, hand_category, HandCategory};
