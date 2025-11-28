# Linera Poker Engine

High-performance, fully deterministic Texas Hold'em poker engine with a complete tournament system, designed to run in blockchain environments (Linera) and high-load backends.

This crate is the **core off-chain logic** for Strix Poker and Linera-based poker dApps.  
It is built as a pure Rust library: no I/O, no global state, no non-determinism — perfect for embedding into smart contracts (`poker-onchain`) or server-side services.

---

## ✨ Key Features

- **Full Cash / Tournament Hand Engine**
  - Streets: Preflop → Flop → Turn → River → Showdown
  - Actions: `Fold`, `Check`, `Call`, `Bet`, `Raise`, `AllIn`
  - Strict betting rules: `current_bet`, `min_raise`, `last_aggressor`, `to_act`
  - Hand status tracking: `Ongoing` / `Finished` with `HandSummary`

- **Advanced Side Pot Logic**
  - Correct side pots for 2 / 3 / 4+ simultaneous all-ins
  - Deterministic payouts
  - Thoroughly tested in dedicated side-pot tests

- **Deterministic RNG**
  - `DeterministicRng` for reproducible simulations and backtests
  - Hash-based reseeding pipeline per table / hand:
    - `new_seed = H(old_seed || table_id || hand_id)`
  - `SystemRng` for non-deterministic runs (if needed)

- **Tournament Engine**
  - Player registration and seating
  - Multi-table structure
  - Balanced seating and rebalance moves
  - Bust logic & finishing places
  - Blind schedules (normal / turbo / custom)
  - Breaks and time-based level progression

- **Time-Control Primitives**
  - `TurnClock`, `TimeBank`, `ExtraTime`
  - Designed for integration with on-chain / backend orchestrators
  - Used to implement timeouts, auto-actions, SittingOut policies

- **Extensive Test Suite**
  - Unit tests for engine, evaluator, RNG
  - Integration tests for full hands and tournaments
  - Stress tests for big tournaments (hundreds–thousands of players)
  - Deterministic replay tests (same seed → same bust log)

---

## 🧱 Project Structure

```text
src/
  domain/          # Core domain types: cards, chips, players, table, blinds, hands
  engine/          # Hand engine: actions, game loop, betting, side pots, history
  eval/            # Hand evaluator and ranking logic
  tournament/      # Tournament model: config, seating, rebalance, bust logic
  time_ctrl/       # Time control primitives: TurnClock, TimeBank, ExtraTime
  infra/           # RNG (DeterministicRng, SystemRng) and utilities
  state.rs         # Engine-level state helpers (for tests / simulations)
  lib.rs           # Crate root

tests/
  engine_actions_tests.rs
  engine_error_tests.rs
  engine_integration_tests.rs
  engine_preflop_tests.rs
  engine_showdown_tests.rs
  engine_sidepots_tests.rs
  engine_stress_tests.rs
  rng_tests.rs
  tournament_balancing_tests.rs
  tournament_blinds_test.rs
  tournament_logic_tests.rs
  tournament_time_tests.rs
