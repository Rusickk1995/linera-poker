# Linera Poker Engine

High-performance, fully deterministic Texas Hold’em engine designed for **Linera** and high-load backends.  
This crate implements the **core off-chain logic** for Strix Poker and Linera-based poker dApps.

> Status: **Active development (private repo)** – API may change.

---

## Table of Contents

- [Overview](#overview)
- [Key Features](#key-features)
- [Architecture](#architecture)
  - [Domain Layer (`src/domain`)](#domain-layer-srcdomain)
  - [Engine Layer (`src/engine`)](#engine-layer-srcengine)
  - [Evaluation Layer (`src/eval`)](#evaluation-layer-srceval)
  - [Infrastructure Layer (`src/infra`)](#infrastructure-layer-srcinfra)
  - [Time Control (`src/time_ctrl`)](#time-control-srctime_ctrl)
  - [Tournament Logic (`src/tournament`)](#tournament-logic-srctournament)
  - [API Layer (`src/api`)](#api-layer-srcapi)
- [Project Structure](#project-structure)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Build](#build)
  - [Run Tests](#run-tests)
  - [Using the Engine in Your Code](#using-the-engine-in-your-code)
- [Deterministic RNG & Fairness](#deterministic-rng--fairness)
- [Time Bank & Turn Clock](#time-bank--turn-clock)
- [Tournaments & Multi-Table Support](#tournaments--multi-table-support)
- [Integration with Linera](#integration-with-linera)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)
- [Russian Summary / Кратко по-русски](#russian-summary--кратко-по-русски)

---

## Overview

`poker-engine` is a **pure Rust** library that implements:

- Full **Texas Hold’em** hand lifecycle (preflop → flop → turn → river → showdown).
- **Cash game** and **tournament** logic (including multi-table MTT).
- Strongly typed **domain model** (cards, chips, players, tables, tournaments).
- Deterministic, blockchain-friendly **RNG pipeline** with hash-reseeding.
- **Time control** (shot clock, time bank, extra time rules).
- Extensive **tests** (unit, integration, stress).

The engine is designed to be:

- **Deterministic** – same inputs (including RNG seed) ⇒ same outputs.
- **Side-effect free** – no I/O, no global state.
- **Portable** – works in:
  - Native backends (Linux/Windows/macOS),
  - Linera smart contracts (via `linera-sdk`, wasm32 target),
  - Any environment that can call into Rust.

---

## Key Features

- **Full Hand Engine**
  - Streets: `Preflop`, `Flop`, `Turn`, `River`, `Showdown`.
  - Actions: `Fold`, `Check`, `Call`, `Bet`, `Raise`, `AllIn`.
  - Strict betting rules: `current_bet`, `min_raise`, `last_aggressor`, `to_act`.
  - Hand lifecycle: `HandStatus::Ongoing` / `HandStatus::Finished(HandSummary, HandHistory)`.

- **Side Pots & All-In Logic**
  - Correct distribution of pots in **multi-way all-in** scenarios.
  - `Pot` + `SidePot` abstractions and `compute_side_pots` helper.
  - Tested edge cases with 2–4 all-in players.

- **Robust Evaluation Engine**
  - Fast 7-card evaluation: player hole cards + board.
  - `HandRank` and `HandCategory` abstraction.
  - Precomputed lookup tables (`lookup_tables.rs`) for straights/flushes, etc.

- **Deterministic RNG Pipeline**
  - `RandomSource` trait in `engine`.
  - `RngSeed` + `DeterministicRng` / `SystemRng` in `infra`.
  - Hash-reseeding: for each hand new seed derived from previous state + domain data.

- **Tournament Engine**
  - `Tournament`, `TournamentConfig`, `BlindsProgression`.
  - Seat balancing across multiple tables.
  - Rebalance logic (`rebalance.rs`) to move players and keep tables fair.
  - Tournament lobby (`lobby.rs`) with simple in-memory management.

- **Time Bank & Shot Clock**
  - `TimeRules` – configuration for base action time, increment, time bank, extra time.
  - `TimeBank` + `ExtraTimeGrant`.
  - `TurnClock` (`clock.rs`) to track deadlines per player and detect timeouts.

- **Well-Structured API Layer**
  - `api/commands.rs` – high-level commands.
  - `api/queries.rs` – read-only queries for UI / GraphQL.
  - `api/dto.rs` – DTOs to serialize state for frontends/backends.

- **Extensive Test Suite**
  - `tests/engine_*` – engine unit & integration tests.
  - `tests/tournament_*` – tournament and balancing.
  - `tests/rng_tests.rs` – RNG determinism & safety.
  - `tests/engine_stress_tests.rs` – stress & robustness.

---

## Architecture

At a high level, the crate is split into several layers:

### Domain Layer (`src/domain`)

Core poker & tournament domain types:

- `card.rs`
  - `Card`, `Rank`, `Suit`.
- `chips.rs`
  - Strongly typed chip representation `Chips`.
- `hand.rs`
  - `HandRank`, `HandSummary`, `PlayerHandResult`, `Street`.
- `player.rs`
  - `PlayerAtTable`, `PlayerStatus`.
- `table.rs`
  - `Table`, `TableConfig`, `TableType`, `TableStakes`, `SeatIndex`.
- `blinds.rs`
  - `AnteType`, blind levels, blind schedule configs for tournaments.
- `tournament.rs`
  - `Tournament`, `TournamentConfig`, `PlayerRegistration`, `TournamentId`, etc.
- `mod.rs`
  - Common type aliases: `PlayerId`, `TableId`, `TournamentId`, `HandId`, etc.

This layer carries **no engine logic**, only state and rules of the domain.

### Engine Layer (`src/engine`)

The core state machine for a single table and hand:

- `mod.rs`
  - Re-exports hand engine API:
    - `start_hand`
    - `apply_action`
    - `advance_if_needed`
    - `HandEngine`
    - `HandStatus`
  - Defines `RandomSource` trait.
  - Re-exports `TableManager` for multi-table management.

- `game_loop.rs`
  - Implementation of `HandEngine` and the full hand lifecycle.
  - Maintains:
    - `Deck`,
    - `BettingState`,
    - `Pot` + `SidePot`,
    - board cards & street transitions,
    - finished hand summary and history.

- `actions.rs`
  - `PlayerAction`, `PlayerActionKind` (`Fold`, `Call`, `Raise`, etc.).
  - Validation and application of actions.

- `betting.rs`
  - `BettingState`, tracking:
    - current bet,
    - minimum raise,
    - `to_act` list,
    - last aggressor, etc.

- `hand_history.rs`
  - `HandHistory`, `HandEvent`, `HandEventKind`.
  - Chronological log of everything that happened in a hand.

- `positions.rs`
  - Helper logic for dealer button, blinds positions, occupied seats.

- `pot.rs` / `side_pots.rs`
  - Main and side pot structures and algorithms.

- `table_manager.rs`
  - Higher-level `TableManager` for orchestrating multiple hands/tables.

- `errors.rs`
  - `EngineError` – all engine-level error types.

### Evaluation Layer (`src/eval`)

Fast and deterministic hand evaluation:

- `hand_rank.rs`
  - `HandCategory` for categorizing hands (High Card, Pair, …, Straight Flush).
- `lookup_tables.rs`
  - Precomputed tables / helpers:
    - `detect_straight`,
    - bit masks for ranks (`RankMask`).
- `evaluator.rs`
  - Calculates `HandRank` from player cards + board.
  - Pure functions, no randomness.

### Infrastructure Layer (`src/infra`)

Integration helpers and cross-cutting concerns:

- `ids.rs`
  - Strongly typed IDs and conversions.
- `mapping.rs`
  - Mapping domain → DTOs / API models.
- `persistence.rs`
  - Abstractions for persistence (in memory, DB, chain state).
- `rng.rs`
  - `SystemRng`, `DeterministicRng` implementations.
  - Adapters to `engine::RandomSource`.
- `rng_seed.rs`
  - `RngSeed` type.
  - Hash-reseeding pipeline (using BLAKE3 / SHA-256).

### Time Control (`src/time_ctrl`)

Per-player time management:

- `time_rules.rs`
  - `TimeRules` with presets (Standard, Turbo, Deep).
- `time_bank.rs`
  - Time bank accounting per player.
- `extra_time.rs`
  - Logic for extra time grants.
- `clock.rs`
  - Turn clock per table:
    - Evaluates `TimeoutState` (no active player, still in time, soft timeout, hard timeout).

### Tournament Logic (`src/tournament`)

Multi-table tournament orchestration:

- `lobby.rs`
  - `TournamentLobby` – in-memory lobby managing tournaments and registrations.
- `runtime.rs`
  - Core tournament runtime:
    - managing ongoing tournaments,
    - integrating RNG,
    - running tournament ticks / steps.
- `rebalance.rs`
  - Logic for computing rebalance moves:
    - moves players between tables to keep them balanced.

### API Layer (`src/api`)

High-level API surface for external callers:

- `commands.rs`
  - Command types for external callers (front-end / GraphQL / Linera operations).
- `queries.rs`
  - Query types for read-only access to engine/tournament state.
- `dto.rs`
  - DTO structs for state representation and serialization.
- `errors.rs`
  - API-level error types (validation, not-found, etc.).
- `mod.rs`
  - Glue module exporting top-level API types.

---

## Project Structure

```text
├── Cargo.lock
├── Cargo.toml
├── README.md
├── src
│   ├── api
│   │   ├── commands.rs
│   │   ├── dto.rs
│   │   ├── errors.rs
│   │   ├── mod.rs
│   │   └── queries.rs
│   ├── bin
│   │   └── _old
│   │       ├── poker_dev_cli.rs
│   │       ├── poker_dev_cli_multitable.rs
│   │       ├── poker_dev_cli_single_table.rs
│   │       └── poker_stress_linera.rs
│   ├── domain
│   │   ├── blinds.rs
│   │   ├── card.rs
│   │   ├── chips.rs
│   │   ├── deck.rs
│   │   ├── hand.rs
│   │   ├── mod.rs
│   │   ├── player.rs
│   │   ├── table.rs
│   │   └── tournament.rs
│   ├── engine
│   │   ├── actions.rs
│   │   ├── betting.rs
│   │   ├── errors.rs
│   │   ├── game_loop.rs
│   │   ├── hand_history.rs
│   │   ├── mod.rs
│   │   ├── positions.rs
│   │   ├── pot.rs
│   │   ├── side_pots.rs
│   │   ├── table_manager.rs
│   │   └── validation.rs
│   ├── eval
│   │   ├── evaluator.rs
│   │   ├── hand_rank.rs
│   │   ├── lookup_tables.rs
│   │   └── mod.rs
│   ├── infra
│   │   ├── ids.rs
│   │   ├── mapping.rs
│   │   ├── mod.rs
│   │   ├── persistence.rs
│   │   ├── rng.rs
│   │   └── rng_seed.rs
│   ├── time_ctrl
│   │   ├── clock.rs
│   │   ├── extra_time.rs
│   │   ├── mod.rs
│   │   ├── time_bank.rs
│   │   └── time_rules.rs
│   ├── tournament
│   │   ├── lobby.rs
│   │   ├── mod.rs
│   │   ├── rebalance.rs
│   │   └── runtime.rs
│   ├── lib.rs
│   └── state.rs
└── tests
    ├── _old
    │   ├── api_test.rs
    │   ├── domain_test.rs
    │   ├── engine_core_test.rs
    │   ├── eval_test.rs
    │   ├── heads_up_scenarios.rs
    │   ├── infra_test.rs
    │   ├── mod.rs
    │   ├── multiway_pot_scenarios.rs
    │   └── tournament_flow.rs
    ├── engine_actions_tests.rs
    ├── engine_error_tests.rs
    ├── engine_integration_tests.rs
    ├── engine_preflop_tests.rs
    ├── engine_showdown_tests.rs
    ├── engine_sidepots_tests.rs
    ├── engine_stress_tests.rs
    ├── rng_tests.rs
    ├── tournament_balancing_tests.rs
    ├── tournament_blinds_test.rs
    ├── tournament_logic_tests.rs
    └── tournament_time_tests.rs
