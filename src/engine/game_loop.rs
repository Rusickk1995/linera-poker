use std::collections::HashMap;

use crate::domain::chips::Chips;
use crate::domain::hand::{HandRank, HandSummary, PlayerHandResult, Street};
use crate::domain::player::{PlayerAtTable, PlayerStatus};
use crate::domain::table::{Table, TableStakes};
use crate::domain::{HandId, PlayerId, SeatIndex, TableId};
use crate::domain::deck::Deck;
use crate::eval::evaluate_best_hand;
use crate::engine::actions::{PlayerAction, PlayerActionKind};
use crate::engine::betting::BettingState;
use crate::engine::errors::EngineError;
use crate::engine::hand_history::{HandEventKind, HandHistory};
use crate::engine::positions::{collect_occupied_seats_from, next_dealer};
use crate::engine::pot::Pot;
use crate::engine::side_pots::{compute_side_pots, SidePot};
use crate::engine::validation::validate_action;
use crate::engine::RandomSource;

/// Статус раздачи для внешнего кода.
pub enum HandStatus {
    Ongoing,
    Finished(HandSummary, HandHistory),
}

/// Внутреннее состояние раздачи.
pub struct HandEngine {
    pub table_id: TableId,
    pub hand_id: HandId,
    pub deck: Deck,
    pub betting: BettingState,
    pub pot: Pot,
    pub side_pots: Vec<SidePot>,
    /// Сколько всего фишек внёс каждый seat (для side pots).
    pub contributions: HashMap<SeatIndex, Chips>,
    /// Чей сейчас ход (seat).
    pub current_actor: Option<SeatIndex>,
    /// История раздачи.
    pub history: HandHistory,
}

impl HandEngine {
    fn new(table_id: TableId, hand_id: HandId, deck: Deck, betting: BettingState) -> Self {
        Self {
            table_id,
            hand_id,
            deck,
            betting,
            pot: Pot::new(),
            side_pots: Vec::new(),
            contributions: HashMap::new(),
            current_actor: None,
            history: HandHistory::new(),
        }
    }
}

/// Старт новой раздачи:
/// - выбирает дилера;
/// - постит блайнды/анте;
/// - раздаёт карманные карты;
/// - настраивает BettingState и current_actor.
pub fn start_hand<R: RandomSource>(
    table: &mut Table,
    rng: &mut R,
    new_hand_id: HandId,
) -> Result<HandEngine, EngineError> {
    if table.hand_in_progress {
        return Err(EngineError::HandAlreadyInProgress);
    }
    if table.seated_count() < 2 {
        return Err(EngineError::NotEnoughPlayers);
    }

    let table_id = table.id;
    let mut deck = Deck::standard_52();
    rng.shuffle(&mut deck.cards);

    // Сброс board/pot/флагов.
    table.board.clear();
    table.total_pot = Chips::ZERO;
    table.current_hand_id = Some(new_hand_id);
    table.street = Street::Preflop;
    table.hand_in_progress = true;

    // Обновляем статусы игроков.
    for seat_opt in table.seats.iter_mut() {
        if let Some(p) = seat_opt {
            if !matches!(p.status, PlayerStatus::Busted | PlayerStatus::SittingOut) {
                p.status = PlayerStatus::Active;
                p.current_bet = Chips::ZERO;
                p.hole_cards.clear();
            }
        }
    }

    // Определяем дилера (кнопку).
    let dealer_seat = next_dealer(table).ok_or(EngineError::NotEnoughPlayers)?;
    table.dealer_button = Some(dealer_seat);

    // Инициализация HandEngine.
    let mut engine = HandEngine::new(
        table_id,
        new_hand_id,
        deck,
        BettingState::new(
            Street::Preflop,
            Chips::ZERO,
            table.config.stakes.big_blind, // min_raise по умолчанию = BB
            Vec::new(),
        ),
    );

    engine.history.push(HandEventKind::HandStarted {
        table_id,
        hand_id: new_hand_id,
    });

    // Постим анте + блайнды, определяем порядок действия.
    post_blinds_and_antes(table, &mut engine, dealer_seat);

    // Раздаём hole cards (по 2 карты).
    deal_hole_cards(table, &mut engine);

    Ok(engine)
}

/// Постинг анте и блайндов.
fn post_blinds_and_antes(table: &mut Table, engine: &mut HandEngine, dealer_seat: SeatIndex) {
    let stakes: TableStakes = table.config.stakes.clone();

    let occupied = collect_occupied_seats_from(table, dealer_seat);
    if occupied.len() < 2 {
        return;
    }

    let sb_seat = occupied[1 % occupied.len()];
    let bb_seat = occupied[2 % occupied.len()];

    let mut ante_events = Vec::new();

    // Анте.
    match stakes.ante_type {
        crate::domain::blinds::AnteType::None => {}
        crate::domain::blinds::AnteType::Classic => {
            for &seat in &occupied {
                if let Some(p) = table.seats[seat as usize].as_mut() {
                    let paid = take_from_stack(p, stakes.ante);
                    add_contribution(engine, seat, paid);
                    ante_events.push((seat, paid));
                }
            }
        }
        crate::domain::blinds::AnteType::BigBlind => {
            // Упрощённо: BB платит ante.
            if let Some(p) = table.seats[bb_seat as usize].as_mut() {
                let paid = take_from_stack(p, stakes.ante);
                add_contribution(engine, bb_seat, paid);
                ante_events.push((bb_seat, paid));
            }
        }
    }

    // Small blind.
    let mut sb_evt = None;
    if let Some(p) = table.seats[sb_seat as usize].as_mut() {
        let paid = take_from_stack(p, stakes.small_blind);
        p.current_bet += paid;
        add_contribution(engine, sb_seat, paid);
        sb_evt = Some((sb_seat, paid));
    }

    // Big blind.
    let mut bb_evt = None;
    if let Some(p) = table.seats[bb_seat as usize].as_mut() {
        let paid = take_from_stack(p, stakes.big_blind);
        p.current_bet += paid;
        add_contribution(engine, bb_seat, paid);
        bb_evt = Some((bb_seat, paid));
    }

    engine.betting.current_bet = stakes.big_blind;
    engine.betting.min_raise = stakes.big_blind;
    engine.betting.last_aggressor = Some(bb_seat);

    engine.history.push(HandEventKind::BlindsPosted {
        dealer: dealer_seat,
        small_blind: sb_evt,
        big_blind: bb_evt,
        ante: ante_events,
    });

    // Кто первый ходит на префлопе?
    // Обычно первый после BB.
    let mut to_act = Vec::new();
    let start_idx = match occupied.iter().position(|&s| s == bb_seat) {
        Some(idx) => (idx + 1) % occupied.len(),
        None => 0,
    };
    for i in 0..occupied.len() {
        let idx = (start_idx + i) % occupied.len();
        let seat = occupied[idx];
        // Пропускаем того, кто уже в all-in или busted.
        if let Some(p) = table.seats[seat as usize].as_ref() {
            if matches!(p.status, PlayerStatus::Active) {
                to_act.push(seat);
            }
        }
    }

    engine.betting.to_act = to_act.clone();
    engine.current_actor = to_act.first().copied();
}

/// Взять из стека не более amount.
fn take_from_stack(player: &mut PlayerAtTable, amount: Chips) -> Chips {
    let real = if player.stack.0 < amount.0 {
        Chips(player.stack.0)
    } else {
        amount
    };
    player.stack -= real;
    real
}

/// Обновить общий pot и contributions.
fn add_contribution(engine: &mut HandEngine, seat: SeatIndex, amount: Chips) {
    if amount.is_zero() {
        return;
    }
    engine.pot.add(amount);
    *engine
        .contributions
        .entry(seat)
        .or_insert(Chips::ZERO) += amount;
}

/// Раздача карманных карт – по 2 карты, по кругу.
fn deal_hole_cards(table: &mut Table, engine: &mut HandEngine) {
    let dealer = table.dealer_button.expect("dealer должен быть задан");
    let order = collect_occupied_seats_from(table, dealer);

    for _round in 0..2 {
        for &seat in &order {
            if let Some(p) = table.seats[seat as usize].as_mut() {
                if let Some(card) = engine.deck.draw_one() {
                    p.hole_cards.push(card);
                    engine.history.push(HandEventKind::HoleCardsDealt {
                        seat,
                        cards: vec![card],
                    });
                }
            }
        }
    }
}

/// Применить действие игрока. Возвращает статус раздачи (идёт / закончилась).
pub fn apply_action(
    table: &mut Table,
    engine: &mut HandEngine,
    action: PlayerAction,
) -> Result<HandStatus, EngineError> {
    if !table.hand_in_progress {
        return Err(EngineError::NoActiveHand);
    }

    // Проверяем, что seat валидный.
    let seat_idx = action.seat as usize;
    if seat_idx >= table.seats.len() {
        return Err(EngineError::InvalidSeat(action.seat));
    }

    // Берём иммутабельную ссылку для проверок (без &mut, чтобы не ловить borrow-конфликты).
    let player_ref = table.seats[seat_idx]
        .as_ref()
        .ok_or(EngineError::EmptySeat)?;

    // Проверяем, что этот игрок реально сидит в этом месте.
    if player_ref.player_id != action.player_id {
        return Err(EngineError::PlayerNotAtTable(action.player_id));
    }

    // Проверяем, что сейчас ход этого seat.
    if engine.current_actor != Some(action.seat) {
        return Err(EngineError::NotPlayersTurn(action.player_id));
    }

    // Валидация действия по текущему состоянию.
    validate_action(player_ref, &action.kind, &engine.betting)?;

    // Сколько нужно доплатить до call – считаем по текущему bet'у игрока.
    let to_call = if engine.betting.current_bet.0 > player_ref.current_bet.0 {
        Chips(engine.betting.current_bet.0 - player_ref.current_bet.0)
    } else {
        Chips::ZERO
    };

    let _stakes = table.config.stakes.clone();

    // Применяем действие.
    match action.kind {
        PlayerActionKind::Fold => {
            let (player_id, new_stack) = {
                let player = table.seats[seat_idx]
                    .as_mut()
                    .ok_or(EngineError::EmptySeat)?;
                player.status = PlayerStatus::Folded;
                (player.player_id, player.stack)
            };

            engine.history.push(HandEventKind::PlayerActed {
                player_id,
                seat: action.seat,
                action: action.kind,
                new_stack,
                pot_after: engine.pot.total,
            });
        }

        PlayerActionKind::Check => {
            let (player_id, new_stack) = {
                let player = table.seats[seat_idx]
                    .as_mut()
                    .ok_or(EngineError::EmptySeat)?;
                (player.player_id, player.stack)
            };

            engine.history.push(HandEventKind::PlayerActed {
                player_id,
                seat: action.seat,
                action: action.kind,
                new_stack,
                pot_after: engine.pot.total,
            });
        }

        PlayerActionKind::Call => {
            let (player_id, new_stack) = {
                let player = table.seats[seat_idx]
                    .as_mut()
                    .ok_or(EngineError::EmptySeat)?;

                let _pay = if player.stack.0 <= to_call.0 {
                    // all-in call
                    let allin = player.stack;
                    player.stack = Chips::ZERO;
                    player.status = PlayerStatus::AllIn;
                    let diff = Chips(player.current_bet.0 + allin.0);
                    let added = Chips(diff.0 - player.current_bet.0);
                    player.current_bet = diff;
                    add_contribution(engine, action.seat, added);
                    allin
                } else {
                    player.stack -= to_call;
                    player.current_bet += to_call;
                    add_contribution(engine, action.seat, to_call);
                    to_call
                };

                (player.player_id, player.stack)
            };

            engine.history.push(HandEventKind::PlayerActed {
                player_id,
                seat: action.seat,
                action: action.kind,
                new_stack,
                pot_after: engine.pot.total,
            });
        }

        PlayerActionKind::Bet(amount) => {
            let (player_id, new_stack, new_bet) = {
                let player = table.seats[seat_idx]
                    .as_mut()
                    .ok_or(EngineError::EmptySeat)?;

                // Сбрасываем предыдущие беты (логика round reset – в engine).
                let diff = if player.stack.0 <= amount.0 {
                    // bet all-in
                    let allin = player.stack;
                    player.stack = Chips::ZERO;
                    player.status = PlayerStatus::AllIn;
                    allin
                } else {
                    // обычный bet: списываем фишки со стека
                    player.stack -= amount;
                    amount
                };

                player.current_bet += diff;
                add_contribution(engine, action.seat, diff);

                (player.player_id, player.stack, player.current_bet)
            };

            // Новый бет → новый current_bet/min_raise.
            engine.betting.on_raise(
                action.seat,
                new_bet,
                amount, // min_raise = bet размер (первый bet)
                collect_betting_order_after_raise(table, action.seat),
            );

            engine.history.push(HandEventKind::PlayerActed {
                player_id,
                seat: action.seat,
                action: action.kind,
                new_stack,
                pot_after: engine.pot.total,
            });
        }

        PlayerActionKind::Raise(total_bet) => {
            let current_bet_before = engine.betting.current_bet;
            let (player_id, new_stack, new_bet) = {
                let player = table.seats[seat_idx]
                    .as_mut()
                    .ok_or(EngineError::EmptySeat)?;

                let diff_to_target = Chips(total_bet.0 - player.current_bet.0);
                let real_diff = if player.stack.0 <= diff_to_target.0 {
                    // all-in raise
                    let allin = player.stack;
                    player.stack = Chips::ZERO;
                    player.status = PlayerStatus::AllIn;
                    allin
                } else {
                    // обычный рейз: списываем фишки со стека
                    player.stack -= diff_to_target;
                    diff_to_target
                };

                player.current_bet += real_diff;
                add_contribution(engine, action.seat, real_diff);

                (player.player_id, player.stack, player.current_bet)
            };

            let raise_size = Chips(new_bet.0 - current_bet_before.0);

            engine.betting.on_raise(
                action.seat,
                new_bet,
                raise_size,
                collect_betting_order_after_raise(table, action.seat),
            );

            engine.history.push(HandEventKind::PlayerActed {
                player_id,
                seat: action.seat,
                action: action.kind,
                new_stack,
                pot_after: engine.pot.total,
            });
        }

        PlayerActionKind::AllIn => {
            let current_bet_before = engine.betting.current_bet;
            let (player_id, new_stack, new_bet) = {
                let player = table.seats[seat_idx]
                    .as_mut()
                    .ok_or(EngineError::EmptySeat)?;

                let allin = player.stack;
                player.stack = Chips::ZERO;
                player.status = PlayerStatus::AllIn;

                let new_bet = Chips(player.current_bet.0 + allin.0);
                let diff = Chips(new_bet.0 - player.current_bet.0);

                player.current_bet = new_bet;
                add_contribution(engine, action.seat, diff);

                (player.player_id, player.stack, new_bet)
            };

            // Если он превысил текущий bet → по сути raise.
            if new_bet.0 > current_bet_before.0 {
                let raise_size = Chips(new_bet.0 - current_bet_before.0);
                engine.betting.on_raise(
                    action.seat,
                    new_bet,
                    raise_size,
                    collect_betting_order_after_raise(table, action.seat),
                );
            } else {
                // all-in call / under-call – просто снимаем из очереди.
                engine.betting.mark_acted(action.seat);
            }

            engine.history.push(HandEventKind::PlayerActed {
                player_id,
                seat: action.seat,
                action: action.kind,
                new_stack,
                pot_after: engine.pot.total,
            });
        }
    }

    // Текущий игрок походил → убираем из очереди.
    engine.betting.mark_acted(action.seat);

    // Проверяем, не остался ли один активный игрок (авто победитель).
    if count_active_players(table) == 1 {
        let summary = finish_hand_without_showdown(table, engine);
        table.hand_in_progress = false;
        return Ok(HandStatus::Finished(summary, engine.history.clone()));
    }

    // Если раунд ставок завершён → переходим на следующую улицу / шоудаун.
    if engine.betting.is_round_complete() {
        advance_if_needed(table, engine)
    } else {
        // Иначе – просто передаём ход следующему из очереди.
        engine.current_actor = engine.betting.to_act.first().copied();
        Ok(HandStatus::Ongoing)
    }
}

/// Пересчёт порядка игроков после рейза:
/// начинаем со следующего за raiser по кругу, включаем только активных/не all-in.
fn collect_betting_order_after_raise(table: &Table, raiser_seat: SeatIndex) -> Vec<SeatIndex> {
    let order = collect_occupied_seats_from(table, raiser_seat);
    let mut res = Vec::new();
    // начиная со следующего
    if order.len() <= 1 {
        return res;
    }

    let start_idx = 1;
    for i in 0..(order.len() - 1) {
        let idx = (start_idx + i) % order.len();
        let seat = order[idx];
        if let Some(p) = table.seats[seat as usize].as_ref() {
            if matches!(p.status, PlayerStatus::Active) {
                res.push(seat);
            }
        }
    }
    res
}

/// Подсчёт активных игроков (не folded/busted, в раздаче).
fn count_active_players(table: &Table) -> usize {
    table
        .seats
        .iter()
        .filter_map(|s| s.as_ref())
        .filter(|p| matches!(p.status, PlayerStatus::Active | PlayerStatus::AllIn))
        .count()
}

/// Переход улиц / шоудаун / завершение раздачи.
pub fn advance_if_needed(
    table: &mut Table,
    engine: &mut HandEngine,
) -> Result<HandStatus, EngineError> {
    use Street::*;

    match table.street {
        Preflop => {
            // Открываем флоп (3 карты).
            deal_board_cards(table, engine, 3, Street::Flop);
            reset_bets_for_new_street(table, engine, Street::Flop);
            Ok(HandStatus::Ongoing)
        }
        Flop => {
            // Turn (1 карта).
            deal_board_cards(table, engine, 1, Street::Turn);
            reset_bets_for_new_street(table, engine, Street::Turn);
            Ok(HandStatus::Ongoing)
        }
        Turn => {
            // River (1 карта).
            deal_board_cards(table, engine, 1, Street::River);
            reset_bets_for_new_street(table, engine, Street::River);
            Ok(HandStatus::Ongoing)
        }
        River => {
            // Шоудаун.
            let summary = finish_hand_with_showdown(table, engine);
            table.hand_in_progress = false;
            Ok(HandStatus::Finished(summary, engine.history.clone()))
        }
        Showdown => {
            // Уже не должно быть сюда перехода – раздача должна быть завершена.
            Err(EngineError::Internal("Попытка advance на Showdown"))
        }
    }
}

/// Открыть board карты.
fn deal_board_cards(table: &mut Table, engine: &mut HandEngine, count: usize, street: Street) {
    for _ in 0..count {
        if let Some(card) = engine.deck.draw_one() {
            table.board.push(card);
        }
    }

    engine.history.push(HandEventKind::BoardDealt {
        street,
        cards: table.board.clone(),
    });

    table.street = street;
    engine.history.push(HandEventKind::StreetChanged { street });
}

/// Сбросить current_bet у игроков, настроить новый BettingState для улицы.
fn reset_bets_for_new_street(table: &mut Table, engine: &mut HandEngine, street: Street) {
    for seat_opt in table.seats.iter_mut() {
        if let Some(p) = seat_opt {
            p.current_bet = Chips::ZERO;
        }
    }

    let occupied = collect_occupied_seats_from(table, table.dealer_button.unwrap());
    let mut to_act = Vec::new();

    // На постфлоп улицах первым ходит первый активный игрок слева от дилера.
    if let Some(first) = occupied
        .iter()
        .find(|&&seat| {
            table.seats[seat as usize]
                .as_ref()
                .map(|p| matches!(p.status, PlayerStatus::Active))
                .unwrap_or(false)
        })
        .copied()
    {
        // формируем очередь
        let mut idx = occupied
            .iter()
            .position(|&s| s == first)
            .unwrap_or(0);
        for _ in 0..occupied.len() {
            let seat = occupied[idx];
            if let Some(p) = table.seats[seat as usize].as_ref() {
                if matches!(p.status, PlayerStatus::Active) {
                    to_act.push(seat);
                }
            }
            idx = (idx + 1) % occupied.len();
        }

        engine.betting = BettingState::new(
            street,
            Chips::ZERO,
            table.config.stakes.big_blind,
            to_act.clone(),
        );
        engine.current_actor = to_act.first().copied();
    } else {
        // Никто не активен – раздача должна завершиться раньше.
        engine.current_actor = None;
    }
}

/// Завершение раздачи без шоудауна (все сфолдили, остался один).
fn finish_hand_without_showdown(table: &mut Table, engine: &mut HandEngine) -> HandSummary {
    table.street = Street::Showdown;

    // Победитель – единственный активный игрок.
    let mut winner_seat = None;
    for (idx, seat_opt) in table.seats.iter().enumerate() {
        if let Some(p) = seat_opt.as_ref() {
            if matches!(p.status, PlayerStatus::Active | PlayerStatus::AllIn) {
                winner_seat = Some(idx as SeatIndex);
                break;
            }
        }
    }

    let winner_seat = winner_seat.expect("должен быть хотя бы один активный игрок");
    let total_pot = engine.pot.total;

    if let Some(winner) = table.seats[winner_seat as usize].as_mut() {
        winner.stack += total_pot;
        engine.history.push(HandEventKind::PotAwarded {
            seat: winner_seat,
            player_id: winner.player_id,
            amount: total_pot,
        });
    }

    engine.history.push(HandEventKind::HandFinished {
        hand_id: engine.hand_id,
        table_id: engine.table_id,
    });

    // Обновляем статусы bust’ов по итогам раздачи.
    update_busted_statuses_after_hand(table);

    table.total_pot = Chips::ZERO;

    HandSummary {
        hand_id: engine.hand_id,
        table_id: engine.table_id,
        street_reached: table.street,
        board: table.board.clone(),
        total_pot,
        results: build_results_single_winner(table, winner_seat, total_pot),
    }
}

/// Завершение раздачи с шоудауном и side pots.
fn finish_hand_with_showdown(table: &mut Table, engine: &mut HandEngine) -> HandSummary {
    table.street = Street::Showdown;

    // Считаем сайд-поты.
    let side_pots = compute_side_pots(&engine.contributions);
    engine.side_pots = side_pots.clone();

    let mut results_map: HashMap<SeatIndex, PlayerHandResult> = HashMap::new();

    // Для каждого pot ищем победителей.
    for sp in &side_pots {
        if sp.amount.is_zero() {
            continue;
        }

        // Кандидаты – те, кто не сфолдил и в раздаче.
        let mut best_rank: Option<HandRank> = None;
        let mut winners: Vec<SeatIndex> = Vec::new();

        for &seat in &sp.eligible_seats {
            let player_opt = table.seats[seat as usize].as_ref();
            if let Some(p) = player_opt {
                if !matches!(p.status, PlayerStatus::Folded | PlayerStatus::Busted) {
                    // Вычисляем силу руки.
                    let rank = evaluate_best_hand(&p.hole_cards, &table.board);
                    engine.history.push(HandEventKind::ShowdownReveal {
                        seat,
                        player_id: p.player_id,
                        hole_cards: p.hole_cards.clone(),
                        rank_value: rank.0,
                    });

                    match best_rank {
                        None => {
                            best_rank = Some(rank);
                            winners.clear();
                            winners.push(seat);
                        }
                        Some(br) => {
                            if rank > br {
                                best_rank = Some(rank);
                                winners.clear();
                                winners.push(seat);
                            } else if rank == br {
                                winners.push(seat);
                            }
                        }
                    }

                    // Обновляем rank в results_map
                    let entry = results_map.entry(seat).or_insert(PlayerHandResult {
                        player_id: p.player_id,
                        rank: Some(rank),
                        net_chips: Chips::ZERO,
                        is_winner: false,
                    });
                    entry.rank = Some(rank);
                }
            }
        }

        if winners.is_empty() {
            continue;
        }

        // Делим pot поровну между победителями.
        let share = Chips(sp.amount.0 / winners.len() as u64);
        let mut remainder = Chips(sp.amount.0 % winners.len() as u64);

        for &seat in &winners {
            if let Some(p) = table.seats[seat as usize].as_mut() {
                let mut prize = share;
                if remainder.0 > 0 {
                    prize.0 += 1;
                    remainder.0 -= 1;
                }
                p.stack += prize;

                engine.history.push(HandEventKind::PotAwarded {
                    seat,
                    player_id: p.player_id,
                    amount: prize,
                });

                let entry = results_map.entry(seat).or_insert(PlayerHandResult {
                    player_id: p.player_id,
                    rank: None,
                    net_chips: Chips::ZERO,
                    is_winner: false,
                });
                entry.net_chips += prize;
                entry.is_winner = true;
            }
        }
    }

    engine.history.push(HandEventKind::HandFinished {
        hand_id: engine.hand_id,
        table_id: engine.table_id,
    });

    // Обновляем статусы bust’ов по итогам раздачи.
    update_busted_statuses_after_hand(table);

    table.total_pot = Chips::ZERO;

    let total_pot = engine.pot.total;
    let mut results: Vec<PlayerHandResult> = results_map.into_values().collect();

    HandSummary {
        hand_id: engine.hand_id,
        table_id: engine.table_id,
        street_reached: table.street,
        board: table.board.clone(),
        total_pot,
        results: {
            results.sort_by_key(|r| r.player_id);
            results
        },
    }
}

/// Результаты при победителе без шоудауна.
fn build_results_single_winner(
    table: &Table,
    winner_seat: SeatIndex,
    total_pot: Chips,
) -> Vec<PlayerHandResult> {
    let mut res = Vec::new();

    for (idx, seat_opt) in table.seats.iter().enumerate() {
        if let Some(p) = seat_opt.as_ref() {
            let seat = idx as SeatIndex;
            let is_winner = seat == winner_seat;
            res.push(PlayerHandResult {
                player_id: p.player_id,
                rank: None,
                net_chips: if is_winner { total_pot } else { Chips::ZERO },
                is_winner,
            });
        }
    }

    res
}

/// Пометить игроков как Busted, если после раздачи у них стек 0.
///
/// Это нужно, чтобы турнирный слой (`Tournament`) или инфраструктура
/// могли однозначно увидеть, кто вылетел с этого стола.
fn update_busted_statuses_after_hand(table: &mut Table) {
    for seat_opt in table.seats.iter_mut() {
        if let Some(p) = seat_opt {
            if p.stack.is_zero()
                && !matches!(p.status, PlayerStatus::Busted | PlayerStatus::SittingOut)
            {
                p.status = PlayerStatus::Busted;
            }
        }
    }
}
