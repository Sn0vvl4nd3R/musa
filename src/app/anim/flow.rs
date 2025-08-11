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

#[inline]
pub(super) fn flow_field(u: f32, v: f32, t: f32, p: &FlowParams) -> f32 {
    let du = (u * 0.9 + 0.6 * t).sin() * (v * 0.7 - 0.4 * t).cos() * p.warp;
    let dv = (u * 0.6 - 0.5 * t).cos() * (v * 0.8 + 0.3 * t).sin() * p.warp;
    let (uu, vv) = (u + du, v + dv);

    let mut acc = 0.0f32;
    let mut norm = 0.0f32;
    for i in 0..4 {
        let phase = (p.kx[i] * uu + p.ky[i] * vv) * std::f32::consts::TAU
                  + (p.w[i] * t) * std::f32::consts::TAU
                  + p.ph[i];
        let s = phase.sin();
        let a = [0.48, 0.36, 0.30, 0.24][i];
        acc += s * a;
        norm += a;
    }
    (acc / norm).clamp(-1.0, 1.0)
}
