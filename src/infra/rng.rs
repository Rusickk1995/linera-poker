//! RNG-реализации и сид-пайплайн для покерного движка.
//!
//! Интерфейс для движка задаётся трейтом `crate::engine::RandomSource`.
//!
//! Идея:
//! - на native (Linux/Windows/macOS) используем `rand::rngs::StdRng`:
//!     - `SystemRng` — от системной энтропии (OsRng);
//!     - `DeterministicRng` — от фиксированного сида (для тестов / реплеев).
//! - на wasm (Linera контракт) не используем `rand`:
//!     - `DeterministicRng` — лёгкий xorshift64* с ручным сидом;
//!     - `SystemRng` — заглушка.
//!
//! + поверх этого вводим `RngSeed` и hash-reseeding пайплайн:
//!     - есть базовый сид `RngSeed`;
//!     - для каждой новой раздачи делаем:
//!         new_seed = H( domain || old_seed || table_id || hand_id || hand_index );
//!         rng = DeterministicRng::from_seed(new_seed);
//!     - это даёт:
//!         * доменную изоляцию,
//!         * воспроизводимость,
//!         * возможность ончейн-зеркала с тем же алгоритмом.

use crate::engine::RandomSource;

/// Базовый сид RNG, который можно хранить в состоянии (off-chain / on-chain)
/// и детерминированно "расширять" на каждую раздачу.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RngSeed(pub [u8; 32]);

impl RngSeed {
    /// Создать `RngSeed` из 32 байтов.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Получить байты сида.
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Создать `RngSeed` из `u64` (удобно для тестов).
    pub fn from_u64(seed: u64) -> Self {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&seed.to_le_bytes());
        Self(bytes)
    }

    /// На native можем сделать сид из системной энтропии.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_entropy() -> Self {
        use rand::{RngCore, SeedableRng};
        use rand::rngs::{OsRng, StdRng};

        let mut seed = <StdRng as SeedableRng>::Seed::default();
        OsRng.fill_bytes(&mut seed);
        Self(seed)
    }

    /// Доменное хэш-расширение для новой раздачи.
    ///
    /// `domain_tag` — строка для логического разделения области (например, `"poker-hand-v1"`),
    /// `table_id`, `hand_id`, `hand_index` — данные, определяющие конкретную раздачу.
    pub fn derive_for_hand(
        &self,
        domain_tag: &str,
        table_id: u64,
        hand_id: u64,
        hand_index: u64,
    ) -> Self {
        use blake3::Hasher;

        let mut h = Hasher::new();

        // Доменная строка — чтобы этот RNG нельзя случайно переиспользовать
        // для чего-то ещё (commitment, tickets и т.п.).
        h.update(domain_tag.as_bytes());

        // Предыдущий сид.
        h.update(&self.0);

        // Идентификаторы стола / раздачи.
        h.update(&table_id.to_le_bytes());
        h.update(&hand_id.to_le_bytes());
        h.update(&hand_index.to_le_bytes());

        let out = h.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(out.as_bytes());
        Self(bytes)
    }

    /// Удобный helper: получить *новый* сид и `DeterministicRng` для конкретной раздачи.
    ///
    /// Используем фиксированную доменную строку `"poker-hand-v1"`, чтобы
    /// на ончейне можно было реализовать тот же алгоритм и получить идентичные результаты.
    pub fn rng_for_hand(
        &self,
        table_id: u64,
        hand_id: u64,
        hand_index: u64,
    ) -> (RngSeed, crate::infra::rng::DeterministicRng) {
        let new_seed = self.derive_for_hand("poker-hand-v1", table_id, hand_id, hand_index);
        let rng = crate::infra::rng::DeterministicRng::from_seed(new_seed.0);
        (new_seed, rng)
    }
}

//
// ========================= NATIVE (НЕ wasm32) =========================
//
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::RandomSource;
    use rand::rngs::{OsRng, StdRng};
    use rand::seq::SliceRandom;
    use rand::{RngCore, SeedableRng};

    /// RNG для обычного запуска (CLI, стресс-тесты, локальный сервер).
    ///
    /// - Использует `StdRng` (псевдо-случайный, криптографически стойкий).
    /// - Сидится от системного `OsRng` по умолчанию.
    #[derive(Clone, Debug)]
    pub struct SystemRng {
        inner: StdRng,
    }

    impl SystemRng {
        /// RNG от системной энтропии (default).
        pub fn from_entropy() -> Self {
            let mut seed = <StdRng as SeedableRng>::Seed::default();
            OsRng.fill_bytes(&mut seed);
            Self {
                inner: StdRng::from_seed(seed),
            }
        }

        /// RNG от конкретного сида (32 байта).
        ///
        /// Удобно для воспроизводимых тестов / реплеев.
        pub fn from_seed(seed: [u8; 32]) -> Self {
            Self {
                inner: StdRng::from_seed(seed),
            }
        }

        /// RNG от `u64`: удобно, если сид хранится как число.
        pub fn from_u64(seed: u64) -> Self {
            let mut bytes = [0u8; 32];
            bytes[..8].copy_from_slice(&seed.to_le_bytes());
            Self::from_seed(bytes)
        }
    }

    impl Default for SystemRng {
        fn default() -> Self {
            Self::from_entropy()
        }
    }

    impl RandomSource for SystemRng {
        fn shuffle<T>(&mut self, slice: &mut [T]) {
            slice.shuffle(&mut self.inner);
        }
    }

    /// Детерминированный RNG для тестов / реплеев / симуляций.
    ///
    /// В отличие от `SystemRng`, **всегда** создаётся от сида.
    #[derive(Clone, Debug)]
    pub struct DeterministicRng {
        inner: StdRng,
    }

    impl DeterministicRng {
        /// Создать детерминированный RNG из 32-байтового сида.
        pub fn from_seed(seed: [u8; 32]) -> Self {
            Self {
                inner: StdRng::from_seed(seed),
            }
        }

        /// Удобный конструктор из `u64`.
        pub fn from_u64(seed: u64) -> Self {
            let mut bytes = [0u8; 32];
            bytes[..8].copy_from_slice(&seed.to_le_bytes());
            Self::from_seed(bytes)
        }
    }

    impl RandomSource for DeterministicRng {
        fn shuffle<T>(&mut self, slice: &mut [T]) {
            slice.shuffle(&mut self.inner);
        }
    }

}

//
// ========================= WASM (Linera, target_arch = "wasm32") =========================
//
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::RandomSource;

    /// Лёгкий детерминированный RNG (xorshift64*).
    ///
    /// ВАЖНО:
    /// - Не криптостойкий, но:
    ///     - полностью детерминированный;
    ///     - не зависит от системных источников случайности;
    ///     - не требует `rand` / `getrandom`.
    /// - В проде честный randomness должен приходить извне (VRF, beacon и т.п.),
    ///   а этот RNG просто превращает сид в последовательность чисел.
    #[derive(Clone, Debug)]
    pub struct DeterministicRng {
        state: u64,
    }

    impl DeterministicRng {
        /// Сжатие 32 байт в одно 64-битное состояние.
        fn fold_seed(seed: [u8; 32]) -> u64 {
            const C: u64 = 0x9E37_79B9_7F4A_7C15;
            let mut acc: u64 = C;

            for chunk in seed.chunks(8) {
                let mut buf = [0u8; 8];
                for (i, b) in chunk.iter().enumerate() {
                    buf[i] = *b;
                }
                let v = u64::from_le_bytes(buf);
                acc ^= v.wrapping_mul(C);
                acc = acc.rotate_left(27);
            }

            if acc == 0 {
                0xCAFEBABE_DEADBEEF
            } else {
                acc
            }
        }

        /// Создать RNG из 32-байтового сида.
        pub fn from_seed(seed: [u8; 32]) -> Self {
            Self {
                state: Self::fold_seed(seed),
            }
        }

        /// Удобный конструктор из `u64`.
        pub fn from_u64(seed: u64) -> Self {
            let s = if seed == 0 {
                0xCAFEBABE_DEADBEEF
            } else {
                seed
            };
            Self { state: s }
        }

        /// Следующее 64-битное псевдо-случайное число.
        fn next_u64(&mut self) -> u64 {
            // xorshift64* (стандартный небольшой генератор).
            let mut x = self.state;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.state = x;
            x.wrapping_mul(0x2545F4914F6CDD1D)
        }
    }

    impl RandomSource for DeterministicRng {
        fn shuffle<T>(&mut self, slice: &mut [T]) {
            // Fisher–Yates (Knuth) shuffle, но на нашем next_u64.
            let mut i = slice.len();
            while i > 1 {
                i -= 1;
                let j = (self.next_u64() % ((i + 1) as u64)) as usize;
                slice.swap(i, j);
            }
        }
    }

    /// Заглушка для "системного" RNG на wasm.
    ///
    /// В контракте **не нужно** использовать `SystemRng`.
    /// Для честной игры:
    /// - храним сид в состоянии контракта;
    /// - при старте раздачи берём его, создаём `DeterministicRng`;
    /// - перемешиваем `Deck`;
    /// - обновляем сид (например, `seed = hash(seed || hand_id)`).
    #[derive(Clone, Debug, Default)]
    pub struct SystemRng;

    impl SystemRng {
        /// На wasm логично вместо SystemRng использовать DeterministicRng,
        /// поэтому даём хелпер.
        pub fn from_seed(seed: [u8; 32]) -> DeterministicRng {
            DeterministicRng::from_seed(seed)
        }

        pub fn from_u64(seed: u64) -> DeterministicRng {
            DeterministicRng::from_u64(seed)
        }
    }

    impl RandomSource for SystemRng {
        fn shuffle<T>(&mut self, _slice: &mut [T]) {
            // no-op. Намеренно: если кто-то на wasm попробует
            // использовать SystemRng напрямую, он получит детерминированную
            // "отсортированную" колоду — это сразу видно как баг.
        }
    }

}

//
// ========================= ПУБЛИЧНЫЙ API =========================
//
#[cfg(not(target_arch = "wasm32"))]
pub use native::{DeterministicRng, SystemRng};

#[cfg(target_arch = "wasm32")]
pub use wasm::{DeterministicRng, SystemRng};
