#[derive(Clone, Copy)]
pub(super) struct FlowParams {
    pub kx: [f32; 4],
    pub ky: [f32; 4],
    pub w: [f32; 4],
    pub ph: [f32; 4],
    pub warp: f32,
}

use super::time_rng::rand01;

pub(super) fn make_params(seed: u32) -> FlowParams {
    let mut kx = [0.0; 4];
    let mut ky = [0.0; 4];
    let mut w = [0.0; 4];
    let mut ph = [0.0; 4];

    for i in 0..4 {
        let si = seed ^ ((i as u32 + 1) * 0x9E37_79B9);
        let ang = rand01(si ^ 0x11) * std::f32::consts::TAU;
        let mag = 0.22 + 0.26 * rand01(si ^ 0x22);
        kx[i] = ang.cos() * mag;
        ky[i] = ang.sin() * mag;
        w[i] = 0.015 + 0.030 * rand01(si ^ 0x33);
        ph[i] = rand01(si ^ 0x44) * std::f32::consts::TAU;
    }

    FlowParams {
        kx,
        ky,
        w,
        ph,
        warp: 0.008 + 0.012 * rand01(seed ^ 0x55)
    }
}
