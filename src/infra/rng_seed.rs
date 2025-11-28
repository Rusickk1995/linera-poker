//! RngSeed — криптографически доменный seed для покерного RNG.
//!
//! Позволяет:
//!   - хранить базовый seed (u64 или [u8;32])
//!   - делать детерминированное hash-reseeding:
//!         new = H(domain || old || table_id || hand_id || hand_index)
//!   - создавать DeterministicRng из seed
//!
//! Это фундаментальный компонент для честного воспроизводимого RNG.

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use crate::infra::rng::DeterministicRng;

/// 32-байтовый seed для RNG.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RngSeed {
    pub bytes: [u8; 32],
}

impl RngSeed {
    /// Создать seed из 32 байт.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Создать seed из u64 (для удобства тестов).
    pub fn from_u64(x: u64) -> Self {
        let mut b = [0u8; 32];
        b[..8].copy_from_slice(&x.to_le_bytes());
        Self { bytes: b }
    }

    /// Доменное хэш-расширение с включением контекста:
    ///   - table_id
    ///   - hand_id
    ///   - hand_index (номер раздачи внутри турнира/стола)
    ///
    /// Пример вызова:
    ///     new_seed = old_seed.derive(table, hand, index)
    pub fn derive(&self, table_id: u64, hand_id: u64, hand_index: u64) -> Self {
        let mut hasher = Sha256::new();

        // Доменный префикс
        hasher.update(b"POKER_ENGINE_RNG_V1");

        // Старый seed
        hasher.update(&self.bytes);

        // Table ID
        hasher.update(&table_id.to_le_bytes());

        // Hand ID
        hasher.update(&hand_id.to_le_bytes());

        // Index (counter)
        hasher.update(&hand_index.to_le_bytes());

        let hash = hasher.finalize();

        let mut out = [0u8; 32];
        out.copy_from_slice(&hash[..32]);

        Self { bytes: out }
    }

    /// Создать DeterministicRng из seed.
    pub fn to_rng(&self) -> DeterministicRng {
        DeterministicRng::from_seed(self.bytes)
    }
}
