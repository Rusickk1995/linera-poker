//! RNG tests for poker-engine
//!
//! Эти тесты проверяют:
//! - детерминированность DeterministicRng
//! - различие seed → различие колод
//! - корректную работу shuffle()
//! - отсутствие повторяющихся карт
//! - стабильность hash-reseeding
//! - корректное формирование RngSeed
//! - работу Deck + shuffle + RandomSource

use poker_engine::infra::{DeterministicRng, SystemRng, RngSeed};
use poker_engine::engine::RandomSource;
use poker_engine::domain::deck::Deck;

fn make_u64_seed(a: u64) -> [u8; 32] {
    let mut s = [0u8; 32];
    s[..8].copy_from_slice(&a.to_le_bytes());
    s
}

//
// TEST 1 — DeterministicRng reproducibility
//
#[test]
fn deterministic_rng_same_seed_same_shuffle() {
    let mut r1 = DeterministicRng::from_seed(make_u64_seed(123));
    let mut r2 = DeterministicRng::from_seed(make_u64_seed(123));

    let mut a: Vec<u32> = (0..52).collect();
    let mut b: Vec<u32> = (0..52).collect();

    r1.shuffle(&mut a);
    r2.shuffle(&mut b);

    assert_eq!(a, b, "Same seed must produce identical shuffle");
}

//
// TEST 2 — different seeds produce different shuffle
//
#[test]
fn deterministic_rng_different_seeds_different_shuffle() {
    let mut r1 = DeterministicRng::from_seed(make_u64_seed(111));
    let mut r2 = DeterministicRng::from_seed(make_u64_seed(222));

    let mut a: Vec<u32> = (0..52).collect();
    let mut b: Vec<u32> = (0..52).collect();

    r1.shuffle(&mut a);
    r2.shuffle(&mut b);

    assert_ne!(a, b, "Different seeds must produce different shuffle");
}

//
// TEST 3 — no duplicate cards after shuffle
//
#[test]
fn shuffle_produces_no_duplicates() {
    let mut rng = DeterministicRng::from_seed(make_u64_seed(555));

    let mut deck = (0..52).collect::<Vec<u32>>();
    rng.shuffle(&mut deck);

    let mut sorted = deck.clone();
    sorted.sort_unstable();
    sorted.dedup();

    assert_eq!(sorted.len(), 52, "Shuffled deck must contain 52 unique cards");
}

//
// TEST 4 — Deck.shuffle + RandomSource works correctly
//
#[test]
fn deck_shuffle_works() {
    let mut deck = Deck::standard_52();
    let mut rng = DeterministicRng::from_seed(make_u64_seed(999));

    rng.shuffle(&mut deck.cards);

    assert_eq!(deck.cards.len(), 52);
    assert_ne!(deck.cards, Deck::standard_52().cards);
}

//
// TEST 5 — SystemRng and DeterministicRng produce different outputs
//
#[test]
fn systemrng_vs_deterministic_rng_are_not_equal() {
    let mut sys = SystemRng::default();
    let mut det = DeterministicRng::from_seed(make_u64_seed(12345));

    let mut a: Vec<u32> = (0..52).collect();
    let mut b: Vec<u32> = (0..52).collect();

    sys.shuffle(&mut a);
    det.shuffle(&mut b);

    assert_ne!(a, b, "SystemRng should differ from deterministic RNG");
}

//
// TEST 6 — Deterministic reseeding hash pipeline works
//
#[test]
fn rngseed_hash_pipeline_changes_seed() {
    let base = RngSeed::from_u64(777);

    let s1 = base.derive(1, 10, 0);
    let s2 = base.derive(1, 10, 1);

    assert_ne!(s1, s2, "Different hand indexes must produce different seeds");

    let s3 = base.derive(2, 10, 0);
    assert_ne!(s1, s3, "Different table_id must produce new seed");
}

//
// TEST 7 — RngSeed → DeterministicRng → shuffle is deterministic
//
#[test]
fn rngseed_deterministic_shuffle() {
    let seed = RngSeed::from_u64(123);

    let mut r1 = seed.to_rng();
    let mut r2 = seed.to_rng();

    let mut a = (0..20).collect::<Vec<u32>>();
    let mut b = (0..20).collect::<Vec<u32>>();

    r1.shuffle(&mut a);
    r2.shuffle(&mut b);

    assert_eq!(a, b);
}

//
// TEST 8 — shuffle on empty slice must not crash
//
#[test]
fn shuffle_empty_slice_ok() {
    let mut rng = DeterministicRng::from_seed(make_u64_seed(42));
    let mut arr: Vec<u32> = vec![];
    rng.shuffle(&mut arr);
    assert!(arr.is_empty());
}

//
// TEST 9 — shuffle on 1-element slice must remain the same
//
#[test]
fn shuffle_one_element_ok() {
    let mut rng = DeterministicRng::from_seed(make_u64_seed(42));
    let mut arr = vec![123];
    rng.shuffle(&mut arr);
    assert_eq!(arr, vec![123]);
}

//
// TEST 10 — 1,000 shuffles must never panic
//
#[test]
fn stress_shuffle_many_times() {
    let mut rng = DeterministicRng::from_seed(make_u64_seed(77777));

    for _ in 0..1000 {
        let mut deck = (0..52).collect::<Vec<u32>>();
        rng.shuffle(&mut deck);

        assert_eq!(deck.len(), 52);
    }
}
