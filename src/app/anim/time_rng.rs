use std::sync::OnceLock;

static START: OnceLock<std::time::Instant> = OnceLock::new();

#[inline]
pub(super) fn time_now() -> f32 {
    let s = START.get_or_init(std::time::Instant::now);
    s.elapsed().as_secs_f32()
}

#[inline]
fn hash1(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

#[inline]
pub(super) fn rand01(seed: u32) -> f32 {
    (hash1(seed) as f32) / (u32::MAX as f32)
}
