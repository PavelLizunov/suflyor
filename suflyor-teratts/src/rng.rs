//! Deterministic seeded noise for the sampler's initial latent.
//!
//! The reference uses `numpy.random.default_rng(seed).standard_normal`; exact
//! sequence parity is not required (the diffusion sampler denoises random
//! noise), only a stable, seedable Gaussian. SplitMix64 + Box–Muller keeps
//! this dep-free.

pub struct Rng {
    state: u64,
    spare: Option<f32>,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }

    /// Next standard-normal sample.
    pub fn next_normal_f32(&mut self) -> f32 {
        if let Some(spare) = self.spare.take() {
            return spare;
        }
        let mut u1 = self.next_f64();
        if u1 <= f64::EPSILON {
            u1 = f64::EPSILON;
        }
        let u2 = self.next_f64();
        let radius = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        let spare = radius * theta.sin();
        self.spare = Some(spare as f32);
        (radius * theta.cos()) as f32
    }

    /// Fill `out` with standard-normal samples.
    pub fn fill_normal_f32(&mut self, out: &mut [f32]) {
        for slot in out.iter_mut() {
            *slot = self.next_normal_f32();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(1234);
        let mut b = Rng::new(1234);
        for _ in 0..64 {
            assert_eq!(a.next_normal_f32(), b.next_normal_f32());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let seq_a: Vec<f32> = (0..8).map(|_| a.next_normal_f32()).collect();
        let seq_b: Vec<f32> = (0..8).map(|_| b.next_normal_f32()).collect();
        assert_ne!(seq_a, seq_b);
    }

    #[test]
    fn distribution_is_sane() {
        let mut rng = Rng::new(42);
        let mut buf = vec![0.0_f32; 10_000];
        rng.fill_normal_f32(&mut buf);
        let mean = buf.iter().sum::<f32>() / buf.len() as f32;
        let var = buf.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / buf.len() as f32;
        assert!(mean.abs() < 0.1, "mean {mean}");
        assert!((var - 1.0).abs() < 0.15, "var {var}");
        assert!(buf.iter().all(|x| x.is_finite()));
    }
}
