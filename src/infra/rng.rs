use crate::engine::RandomSource;

//
// ✅ NATIVE ВАРИАНТ (НЕ wasm32):
//    тут есть rand, всё как у тебя было.
//
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Default)]
pub struct SystemRng;

#[cfg(not(target_arch = "wasm32"))]
impl RandomSource for SystemRng {
    fn shuffle<T>(&mut self, slice: &mut [T]) {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        slice.shuffle(&mut thread_rng());
    }
}

/// Детерминированный RNG для тестов и реплея.
/// Позволяет воспроизводить одни и те же раздачи при одинаковом seed.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct DeterministicRng {
    inner: rand::rngs::StdRng,
}

#[cfg(not(target_arch = "wasm32"))]
impl DeterministicRng {
    pub fn from_seed(seed: u64) -> Self {
        use rand::SeedableRng;
        Self {
            inner: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl RandomSource for DeterministicRng {
    fn shuffle<T>(&mut self, slice: &mut [T]) {
        use rand::seq::SliceRandom;
        slice.shuffle(&mut self.inner);
    }
}

//
// ✅ WASM ВАРИАНТ (Linera контракт):
//    тут НЕТ rand / getrandom / wasm-bindgen.
//
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, Default)]
pub struct SystemRng;

#[cfg(target_arch = "wasm32")]
impl RandomSource for SystemRng {
    fn shuffle<T>(&mut self, _slice: &mut [T]) {
        // На wasm пока заглушка: не перемешиваем массив.
        // Дек не будет рандомным, но:
        // - контракт детерминированный,
        // - нет rand/getrandom/wasm-bindgen.
    }
}

// DeterministicRng под wasm не нужен, можно просто не определять.
