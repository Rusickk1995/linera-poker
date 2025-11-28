Understood.
Below is the **full, clean, professional English README**, preserving your structure, tone, and “big company” style.
This is the **final polished version**, ready to replace your repository’s README.md.

---

# **README.md — Production-Grade**

````markdown
# Linera Poker Engine

An industrial-grade, deterministic, high-performance Texas Hold’em engine designed for:

- on-chain smart contracts on **Linera**,
- off-chain Rust backends,
- poker frontends (via API / GraphQL / DTO),
- multi-table tournaments, MTT, timebank, shot-clock, deterministic RNG.

This engine is the computational core of **Strix Poker** and is shared across
local simulations, backend servers, and on-chain logic.

> Status: Active development (internal/private). API is subject to change.

---

## Table of Contents

- [About](#about)
- [Key Features](#key-features)
- [Architecture](#architecture)
- [Cloning & Setup](#cloning--setup)
- [Build](#build)
- [Running Tests](#running-tests)
- [Usage Examples](#usage-examples)
  - [1. Single-Table Hand Example](#1-singletable-hand-example)
  - [2. Tournament / MTT Example (Conceptual)](#2-tournament--mtt-example-conceptual)
- [Deterministic RNG & Fairness](#deterministic-rng--fairness)
- [Timebank & Time Control](#timebank--time-control)
- [Tournaments & Multi-Table](#tournaments--multitable)
- [Linera Integration](#linera-integration)
- [Roadmap](#roadmap)
- [License](#license)

---

## About

**Linera Poker Engine** is a deterministic professional Texas Hold’em engine featuring:

- Full hand lifecycle: preflop → flop → turn → river → showdown.
- Cash games and tournaments (including multi-table MTT).
- Strong domain types (cards, chips, players, tables, tournaments).
- Blockchain-friendly deterministic RNG with hash-reseeding.
- Full time control system: timebank, shot-clock, extra time.
- Comprehensive test suite: unit, integration, and stress tests.

The objective is to provide an **enterprise-grade poker core** suitable for
centralized servers and decentralized on-chain execution.

---

## Key Features

### Full Poker Engine
- Streets: Preflop, Flop, Turn, River, Showdown.
- Actions: Fold, Check, Call, Bet, Raise, All-In.
- Strict betting rules: `current_bet`, `min_raise`, `to_act`, `last_aggressor`.
- Hand outcomes: `HandStatus::Ongoing` / `HandStatus::Finished(HandSummary, HandHistory)`.

### Side-pots & All-in Logic
- Supports any multi-way all-in scenario.
- Main pot + multiple side pots with correct distribution.
- Fully covered by edge-case tests (2–4 all-ins).

### Hand Evaluation (Eval)
- Fast 7-card evaluator (2 hole cards + 5 board cards).
- All hand categories (High Card → Straight Flush).
- Precomputed lookup tables for straights, bitmasks, flush detection.

### Deterministic RNG
- Engine-level `RandomSource` interface.
- `RngSeed` + `DeterministicRng` with hash-reseeding.
- 100% reproducible simulations and tournament replays.

### Tournament Engine
- `Tournament`, `TournamentConfig`, blind levels.
- Seat balancing across tables.
- Player rebalancing (`rebalance`).
- `TournamentLobby` and `runtime` to control entire tournament lifecycle.

### Time Control
- Timebank, extra time, base time, increments.
- Soft/hard timeout evaluation through `TurnClock`.

### API Layer
- Commands, queries, DTOs.
- Error handling tuned for backend/frontend integration.
- Ideal for UI, GraphQL, and on-chain wrappers.

---

## Architecture

The project is separated into several layers:

### `src/domain`
Pure domain logic:
- Cards, chips, players, tables
- Tournaments, blind levels, IDs
- No game logic — only data structures

### `src/engine`
The core state machine:
- `HandEngine`, `game_loop`
- Actions, betting, pots, side-pots
- Hand history, seat positions
- Multi-table management

### `src/eval`
Hand strength evaluation:
- Categories
- Straight masks
- 7-card evaluation functions

### `src/infra`
Infrastructure components:
- RNG implementations
- RngSeed + hash-reseeding
- Persistence abstractions
- Domain ↔ DTO mappings

### `src/time_ctrl`
Time management:
- `TimeRules`, `TimeBank`, `ExtraTime`
- `TurnClock`, timeouts

### `src/tournament`
Tournament engine:
- Lobby
- Runtime
- Balancing and rebalancing

### `src/api`
Public API layer:
- Commands
- Queries
- DTOs
- API errors

---

## Cloning & Setup

Clone the repository:

```bash
git clone https://github.com/Rusickk1995/linera-poker.git
cd linera-poker
````

Check Rust installation:

```bash
rustc --version
cargo --version
```

---

## Build

Standard build:

```bash
cargo build
```

Release build:

```bash
cargo build --release
```

---

## Running Tests

Full test suite:

```bash
cargo test
```

Specific groups:

```bash
# RNG
cargo test rng_tests

# Engine
cargo test engine_actions_tests
cargo test engine_preflop_tests
cargo test engine_showdown_tests
cargo test engine_sidepots_tests

# Tournament
cargo test tournament_logic_tests
cargo test tournament_balancing_tests
cargo test tournament_blinds_test
cargo test tournament_time_tests

# Integration
cargo test engine_integration_tests

# Stress
cargo test engine_stress_tests -- --nocapture
```

---

## Usage Examples

Below are practical examples of how to use the engine.

---

### 1. Single-table hand example

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
    // 1. Stakes
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

    // Seat players (example)
    // table.seat_player(...);

    // 2. Deterministic RNG
    let seed = RngSeed::from_u64(123456789);
    let rng = DeterministicRng::from_seed(seed);

    // 3. Start hand
    let mut engine: HandEngine = start_hand(&mut table, rng).expect("start hand");

    // 4. Example action: Call
    // apply_action(&mut engine, PlayerAction::new(player_id, PlayerActionKind::Call)).unwrap();

    // 5. Advance state
    let status = advance_if_needed(&mut engine);

    match status {
        HandStatus::Ongoing => {
            // Waiting for next actions
        }
        HandStatus::Finished(summary, history) => {
            println!("Hand finished: {:?}", summary);
            println!("Events: {:?}", history.events());
        }
    }
}
```

---

### 2. Tournament / MTT Example (Conceptual)

```rust
use poker_engine::tournament::lobby::TournamentLobby;
use poker_engine::tournament::runtime::TournamentRuntime;
use poker_engine::infra::rng::{DeterministicRng};
use poker_engine::infra::rng_seed::RngSeed;

fn simulate_tournament() {
    let seed = RngSeed::from_u64(42);
    let rng = DeterministicRng::from_seed(seed);

    // 1. Create lobby & tournament
    let mut lobby = TournamentLobby::new();
    let tournament_id = lobby.create_tournament(/* config */);

    // 2. Register players
    // lobby.register_player(tournament_id, player_id)?;

    // 3. Create runtime
    let mut runtime = TournamentRuntime::new(lobby, rng);

    // 4. Progress by ticks
    // while !runtime.is_finished(tournament_id) {
    //     runtime.tick(tournament_id).unwrap();
    // }

    // 5. Inspect results
    // let result = runtime.tournament_result(tournament_id);
}
```

---

## Deterministic RNG & Fairness

The engine follows strict determinism:

```
new_seed = H(prev_seed || table_id || hand_id || hand_index || ...)
```

This approach enables:

* reproducible tournaments,
* verifiable fairness,
* on-chain compatibility,
* full audit replay.

No global randomness is ever used.

---

## Timebank & Time Control

The module `time_ctrl` implements:

* Base time per decision
* Timebank accumulation & spending
* Extra-time grants
* Soft timeout (auto-check / auto-fold)
* Hard timeout (forced fold / elimination)
* `TurnClock` for evaluating current time state

A complete system suitable for professional poker clients and real-time blockchain games.

---

## Tournaments & Multi-Table

The engine supports large-scale tournament workflows:

* Player registration
* Initial seat allocation
* Blind progression
* Table balancing and rebalancing
* Multi-table orchestration
* Deterministic simulations (stress testing)

Used for both backend infrastructure and potential on-chain implementations.

---

## Linera Integration

This engine is designed **specifically** with Linera in mind:

* deterministic state machine,
* clean side-effect-free logic,
* ABI-compatible types,
* easy integration via a dedicated on-chain crate (`poker-onchain`),
* suitable for GraphQL services & UI frontends.

Architecture:

* This repository → **off-chain core engine**
* `poker-onchain` → wasm contract & service built on top of this engine
* Frontend (Strix Poker UI) → communicates via Linera GraphQL or backend wrapper

---

## Roadmap

* 6-9 Hold’em / mixed variants
* Progressive bounty / PKO formats
* Commit–reveal RNG / multi-party randomness
* Performance optimizations
* Full documentation & cookbook

---

## License

This repository is private.
All rights reserved.

To open-source the project, a LICENSE file (MIT/Apache/etc.) should be added.

---

```

---

If you want an **even more marketing-polished**, “Silicon-Valley-grade” README (like Polygon, Aptos, NEAR style), or a **luxury design version with banners & badges**, I can produce that too.
```
