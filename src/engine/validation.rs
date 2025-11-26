use crate::domain::chips::Chips;
use crate::domain::player::{PlayerAtTable, PlayerStatus};
use crate::engine::actions::PlayerActionKind;
use crate::engine::betting::BettingState;
use crate::engine::errors::EngineError;

/// Проверка, может ли игрок выполнить это действие при текущем состоянии ставок.
pub fn validate_action(
    player: &PlayerAtTable,
    action: &PlayerActionKind,
    betting: &BettingState,
) -> Result<(), EngineError> {
    if matches!(player.status, PlayerStatus::Folded | PlayerStatus::Busted | PlayerStatus::SittingOut)
    {
        return Err(EngineError::IllegalAction);
    }

    let stack = player.stack;
    let to_call = diff_to_call(player, betting);

    match action {
        PlayerActionKind::Fold => Ok(()),

        PlayerActionKind::Check => {
            if betting.current_bet.0 == player.current_bet.0 {
                Ok(())
            } else {
                Err(EngineError::CannotCheck)
            }
        }

        PlayerActionKind::Call => {
            if to_call.is_zero() {
                Err(EngineError::CannotCall)
            } else if stack.0 < to_call.0 {
                // Call, но по факту это будет all-in call – всё равно разрешаем,
                // но логика обработки будет в engine.
                Ok(())
            } else {
                Ok(())
            }
        }

        PlayerActionKind::Bet(amount) => {
            if betting.current_bet.0 > 0 {
                return Err(EngineError::IllegalAction); // bet можно только когда ещё нет ставки
            }
            if stack.0 < amount.0 {
                return Err(EngineError::NotEnoughChips);
            }
            if amount.is_zero() {
                return Err(EngineError::IllegalAction);
            }
            Ok(())
        }

        PlayerActionKind::Raise(total_bet) => {
            if betting.current_bet.0 == 0 {
                // Когда нет ставки – это bet, а не raise
                return Err(EngineError::IllegalAction);
            }

            let to_call = to_call;
            if total_bet.0 <= betting.current_bet.0 {
                return Err(EngineError::IllegalAction);
            }

            let raise_size = Chips(total_bet.0 - betting.current_bet.0);

            if raise_size.0 < betting.min_raise.0 {
                return Err(EngineError::RaiseTooSmall);
            }

            let diff = Chips(total_bet.0 - player.current_bet.0);
            if stack.0 < diff.0 {
                return Err(EngineError::NotEnoughChips);
            }

            Ok(())
        }

        PlayerActionKind::AllIn => {
            if stack.is_zero() {
                return Err(EngineError::IllegalAction);
            }
            Ok(())
        }
    }
}

/// Сколько фишек нужно добавить игроку, чтобы уравнять текущую ставку.
fn diff_to_call(player: &PlayerAtTable, betting: &BettingState) -> Chips {
    if betting.current_bet.0 <= player.current_bet.0 {
        Chips::ZERO
    } else {
        Chips(betting.current_bet.0 - player.current_bet.0)
    }
}
