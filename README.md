Отлично, это как раз тот кейс, который легко чинится, просто шагов несколько.

Сейчас у тебя:

* конфликт только в `README.md`,
* плюс локально изменены `src/engine/game_loop.rs` и `tests/engine_preflop_tests.rs` (но **без конфликтов**, просто изменения).

Сделаем так:

---

## 1. Открываем и чиним README.md

Выполни:

```bash
cd ~/projects/linera-dev/linera-poker
nano README.md
```

Внутри ты увидишь что-то типа:

```text
<<<<<<< HEAD
... твой новый README или часть ...
=======
... версия README с GitHub ...
>>>>>>> 1016221...
```

Тебе нужно:

1. **Полностью удалить весь старый текст и конфликтные маркеры**, вообще всё.
2. Вставить туда **целиком ПРЕМИУМ README**, который мы сделали.

Вот он, скопируй от `<p align="center">` до самого конца и вставь в `README.md`:

````markdown
<p align="center">
  <strong style="font-size: 32px;">LINERA POKER ENGINE</strong>
</p>

<p align="center">
  Industrial-grade, deterministic Texas Hold’em engine for Linera and high-load backends.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Language: Rust" />
  <img src="https://img.shields.io/badge/status-active%20development-brightgreen.svg" alt="Status: Active Development" />
  <img src="https://img.shields.io/badge/platform-Linera%20&%20Offchain-blue.svg" alt="Platform: Linera & Off-chain" />
</p>

---

## Overview

**Linera Poker Engine** is a deterministic, production-ready Texas Hold’em engine designed to run:

- as core logic for **on-chain** smart contracts on Linera,
- inside **off-chain** Rust backends and services,
- behind modern poker frontends (API / GraphQL / DTO),
- in large-scale multi-table tournaments with timebank and shot-clock.

The same engine can power local simulations, backend infrastructure, and on-chain execution.

> Status: **Active development (internal/private)** – public API may change.

---

## Table of Contents

- [Overview](#overview)
- [Key Features](#key-features)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
  - [Cloning & Setup](#cloning--setup)
  - [Requirements](#requirements)
- [Build](#build)
- [Running Tests](#running-tests)
- [Usage Examples](#usage-examples)
  - [Single-Table Hand Example](#singletable-hand-example)
  - [Tournament / MTT Example (Conceptual)](#tournament--mtt-example-conceptual)
- [Deterministic RNG & Fairness](#deterministic-rng--fairness)
- [Timebank & Time Control](#timebank--time-control)
- [Tournaments & Multi-Table](#tournaments--multitable)
- [Linera Integration](#linera-integration)
- [Roadmap](#roadmap)
- [Project Status](#project-status)
- [License](#license)

---

## Key Features

### Full Poker Engine

- Streets: **Preflop → Flop → Turn → River → Showdown**.
- Actions: `Fold`, `Check`, `Call`, `Bet`, `Raise`, `AllIn`.
- Strict betting rules (`current_bet`, `min_raise`, `to_act`, `last_aggressor`).
- Clear hand outcomes: `HandStatus::Ongoing` / `HandStatus::Finished(HandSummary, HandHistory)`.

### Side-pots & All-in Handling

- Supports arbitrary **multi-way all-in** scenarios.
- Main pot + multiple side pots with correct chip allocation.
- Edge cases (2–4 all-ins, equal stacks, split pots) covered by tests.

### Hand Evaluation (Eval)

- Fast **7-card evaluator** (2 hole cards + 5 community cards).
- All standard hand categories (High Card → Straight Flush).
- Precomputed lookup tables and bitmasks for efficient evaluation.

### Deterministic RNG

- Engine-level `RandomSource` trait (no global RNG).
- `RngSeed` + `DeterministicRng` with **hash-reseeding per hand**.
- 100% reproducible simulations and deterministic tournament runs.

### Tournament Engine

- `Tournament`, `TournamentConfig`, blind levels and structures.
- Seat balancing across tables, automatic rebalancing.
- `TournamentLobby` and `TournamentRuntime` orchestration.

### Time Control

- `TimeRules`, `TimeBank`, `ExtraTime`, `TurnClock`.
- Configurable base time, increments, timebank, soft/hard timeouts.
- Suitable for real-money, high-pressure environments.

### API Layer

- Command and query types for external callers.
- DTOs for UI / GraphQL / REST.
- API-level error modeling.

---

## Architecture

The repository is organized into clean, testable layers:

### `src/domain`

Domain model only, no engine logic:

- Cards, ranks, suits.
- Chips and stack representation.
- Players, seats, tables, tournaments, blinds.
- Strongly typed IDs (`PlayerId`, `TableId`, `TournamentId`, `HandId`, etc.).

### `src/engine`

Core poker engine:

- `HandEngine` and full hand state machine (`game_loop`).
- Action processing, betting rules, validation.
- Main pot + side pots.
- Seat positions, blinds, dealer button.
- Hand history and event log.
- Multi-table management helpers.

### `src/eval`

Hand-evaluation subsystem:

- Hand categories and ranking.
- Lookup tables and bit-level helpers.
- Deterministic 7-card evaluation.

### `src/infra`

Infrastructure utilities:

- RNG implementations (`DeterministicRng`, system RNG).
- `RngSeed` and hash-reseeding pipeline.
- Mapping between domain objects and DTOs.
- Persistence abstractions.

### `src/time_ctrl`

Time control engine:

- Time rules presets.
- Timebank management.
- Extra-time triggers.
- Turn clock for soft/hard timeouts.

### `src/tournament`

Tournament orchestration:

- Lobby for tournament creation and player registration.
- Runtime for driving tournament progress.
- Balancing and rebalancing between tables.

### `src/api`

Public-facing API layer:

- Commands (mutating operations).
- Queries (read-only state access).
- DTOs and error types.

---

## Getting Started

### Cloning & Setup

```bash
git clone https://github.com/Rusickk1995/linera-poker.git
cd linera-poker
````

If the repository is private, GitHub access is required.

### Requirements

* **Rust** stable, recommended version `1.70+`
* Cargo (bundled with Rust)
* For Linera on-chain usage in a separate crate:

  * target `wasm32-unknown-unknown`

You can check your toolchain with:

```bash
rustc --version
cargo --version
```

---

## Build

Standard debug build:

```bash
cargo build
```

Optimized release build:

```bash
cargo build --release
```

---

## Running Tests

Run the full suite:

```bash
cargo test
```

Run specific test groups:

```bash
# RNG
cargo test rng_tests

# Engine (actions, streets, showdown, side pots)
cargo test engine_actions_tests
cargo test engine_preflop_tests
cargo test engine_showdown_tests
cargo test engine_sidepots_tests

# Tournament logic (flow, balancing, blinds, time)
cargo test tournament_logic_tests
cargo test tournament_balancing_tests
cargo test tournament_blinds_test
cargo test tournament_time_tests

# Integration (engine + tournament + RNG)
cargo test engine_integration_tests

# Stress (large simulations / tournaments)
cargo test engine_stress_tests -- --nocapture
```

---

## Usage Examples

Below are high-level examples of how to embed the engine into your own code.

> Note: actual type names or module paths may evolve – refer to the latest source as the single source of truth.

### Single-Table Hand Example

```rust
use poker_engine::domain::chips::Chips;
use poker_engine::domain::table::{Table, TableConfig, TableStakes, TableType};
use poker_engine::engine::{
    start_hand, apply_action, advance_if_needed,
    HandEngine, HandStatus, PlayerAction, PlayerActionKind,
};
use poker_engine::infra::rng::DeterministicRng;
use poker_engine::infra::rng_seed::RngSeed;

fn play_single_hand_example() {
    // 1. Configure table stakes
    let stakes = TableStakes {
        small_blind: Chips::from_u64(50),
        big_blind: Chips::from_u64(100),
        ante: Chips::zero(),
    };

    let config = TableConfig {
        max_seats: 9,
        table_type: TableType::Cash,
        stakes,
        allow_straddle: false,
    };

    let mut table = Table::new(config);

    // Seat players (example, real implementation depends on your app)
    // table.seat_player(player_id_1, initial_stack_1, seat_index_1)?;
    // table.seat_player(player_id_2, initial_stack_2, seat_index_2)?;
    // ...

    // 2. Prepare deterministic RNG
    let seed = RngSeed::from_u64(123_456_789);
    let rng = DeterministicRng::from_seed(seed);

    // 3. Start a new hand
    let mut engine: HandEngine = start_hand(&mut table, rng).expect("failed to start hand");

    // 4. Apply some actions
    // let action = PlayerAction::new(player_id_1, PlayerActionKind::Call);
    // apply_action(&mut engine, action).expect("invalid action");

    // 5. Advance the engine (street transitions, showdown, etc.)
    let status = advance_if_needed(&mut engine);

    match status {
        HandStatus::Ongoing => {
            // Hand is still in progress: await more actions from players
        }
        HandStatus::Finished(summary, history) => {
            // Distribute pots, update stacks, persist and/or broadcast hand history
            println!("Hand finished: {:?}", summary);
            println!("Events: {:?}", history.events());
        }
    }
}
```

---

### Tournament / MTT Example (Conceptual)

```rust
use poker_engine::tournament::lobby::TournamentLobby;
use poker_engine::tournament::runtime::TournamentRuntime;
use poker_engine::infra::rng::DeterministicRng;
use poker_engine::infra::rng_seed::RngSeed;

fn simulate_tournament() {
    let seed = RngSeed::from_u64(42);
    let rng = DeterministicRng::from_seed(seed);

    // 1. Create lobby and tournament
    let mut lobby = TournamentLobby::new();
    let tournament_id = lobby.create_tournament(/* tournament config */);

    // 2. Register players
    // for player_id in players {
    //     lobby.register_player(tournament_id, player_id).expect("registration failed");
    // }

    // 3. Create runtime
    let mut runtime = TournamentRuntime::new(lobby, rng);

    // 4. Drive the tournament by ticks
    // while !runtime.is_finished(tournament_id) {
    //     runtime.tick(tournament_id).expect("runtime step failed");
    // }

    // 5. Inspect final results
    // let result = runtime.tournament_result(tournament_id);
    // println!("Tournament result: {:?}", result);
}
```

---

## Deterministic RNG & Fairness

The engine is built around deterministic randomness suitable for blockchain and audit scenarios.

A typical reseeding scheme for each new hand:

```text
new_seed = H(prev_seed || table_id || hand_id || hand_index || ...)
```

Where `H` is a secure hash function.

This design provides:

* reproducible tournaments,
* verifiable fairness,
* consistent behavior across environments,
* easy replay for dispute resolution.

No global mutable RNG is ever used inside the engine.

---

## Timebank & Time Control

The `time_ctrl` module offers a complete time management system:

* Base time per decision.
* Timebank accumulation and consumption.
* Extra-time grants for critical spots.
* Soft timeouts (e.g. auto-check / auto-fold).
* Hard timeouts (forced fold, seat removal).
* `TurnClock` abstraction to evaluate current state for each player.

This can be wired into:

* backend logic (enforcement),
* frontend UI timers,
* on-chain constraints at the service layer.

---

## Tournaments & Multi-Table

The tournament engine is designed for **multi-table MTT** scenarios:

* Player registration and seeding.
* Initial seat allocation across multiple tables.
* Blind level progression.
* Automatic balancing and rebalancing to keep tables even.
* Integration with the hand engine at each table.

You can:

* simulate thousands of tournaments locally,
* run them off-chain with deterministic RNG,
* or map the same flow onto Linera.

---

## Linera Integration

The engine is built with **Linera** in mind:

* deterministic, side-effect-free logic,
* separation between core engine and on-chain binding,
* ABI-friendly data structures for contract and service layers.

Typical architecture:

* This repository: **core engine crate**.
* Separate `poker-onchain` crate: Linera contract + service using this engine and `linera-sdk`.
* Frontend (e.g. Strix Poker UI): communicates via Linera GraphQL / RPC or a backend wrapper.

This separation allows shared logic across:

* local development,
* off-chain infrastructure,
* on-chain deployments.

---

## Roadmap

Planned / potential future work:

* Additional variants: Omaha, 6+ Hold’em, mixed games.
* Advanced tournament formats: bounty / PKO / progressive payouts.
* Commit–reveal and multi-party RNG schemes.
* Performance tuning and profiling.
* Extended documentation and examples (cookbook-style).

---

## Project Status

* Scope: **private / internal**.
* Direction: active development, architecture already aligned with Web3 / Linera needs.
* API: may change as integration with frontends and on-chain logic evolves.

Contributions can be coordinated internally (branches, PRs, code review, CI).

---

## License

This repository is currently **closed-source**.

* Default: **All rights reserved**.

If the project is later open-sourced, a `LICENSE` file (e.g. MIT / Apache-2.0) should be added and this section updated accordingly.

````
