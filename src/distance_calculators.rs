#[cfg(feature = "simd")]
use std::simd::f32x4;

pub trait DistanceCalculator {
    fn calc_dist(&mut self, a: &[f32], b: &[f32]) -> f32;
}

pub struct SimpleDotProduct {}
impl DistanceCalculator for SimpleDotProduct {
    fn calc_dist(&mut self, a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(a, b)| a * b).sum()
    }
}

// TODO:
#[cfg(feature = "simd")]
pub struct SimdDotProduct {
    sum_buffer: f32x4,
}
#[cfg(feature = "simd")]
impl DistanceCalculator for SimdDotProduct {
    fn calc_dist(&mut self, a: &[f32], b: &[f32]) -> f32 {
        let a_chunks = a.chunks_exact(4);
        let b_chunks = b.chunks_exact(4);
    }
}
