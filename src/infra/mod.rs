//! Инфраструктурный слой вокруг покерного движка:
//! - генерация ID;
//! - RNG-реализации для движка;
//! - абстракция хранения (off-chain / тесты);
//! - маппинги между API и domain.

pub mod ids;
pub mod mapping;
pub mod rng;
pub mod rng_seed;

pub use ids::*;
pub use mapping::*;
pub use rng::*;
pub use rng_seed::RngSeed;
