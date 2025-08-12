use std::cmp::Ordering;
use crate::app::anim::flow::FlowParams;

const A0: f32 = 0.48;
const A1: f32 = 0.36;
const A2: f32 = 0.30;
const A3: f32 = 0.24;
const INV_A_SUM: f32 = 1.0 / (A0 + A1 + A2 + A3);

#[inline(always)]
pub(super) fn smoothstep_inv(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    1.0 - s
}

#[inline(always)]
pub(super) fn mirror01(x: f32) -> f32 {
    let y = x.rem_euclid(2.0);
    if y <= 1.0 {
        y
    } else {
        2.0 - y
    }
}

#[inline(always)]
pub(super) fn precompute_wtph(p: &FlowParams, t: f32) -> [f32; 4] {
    let mut a = [0.0; 4];
    for i in 0..4 {
        a[i] = (p.w[i] * t) * std::f32::consts::TAU + p.ph[i];
    }
    a
}

#[inline(always)]
pub(super) fn flow_four_sines(uu: f32, vv: f32, p: &FlowParams, wtph: &[f32; 4]) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut acc = 0.0f32;
    acc += ( (p.kx[0] * uu + p.ky[0] * vv) * tau + wtph[0] ).sin() * A0;
    acc += ( (p.kx[1] * uu + p.ky[1] * vv) * tau + wtph[1] ).sin() * A1;
    acc += ( (p.kx[2] * uu + p.ky[2] * vv) * tau + wtph[2] ).sin() * A2;
    acc += ( (p.kx[3] * uu + p.ky[3] * vv) * tau + wtph[3] ).sin() * A3;
    (acc * INV_A_SUM).clamp(-1.0, 1.0)
}

#[inline(always)]
pub(super) fn p90_abs(mut v: Vec<f32>) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let k = ((v.len() as f32) * 0.90).floor() as usize;
    let k = k.min(v.len() - 1);
    let (_, nth, _) = v.select_nth_unstable_by(k, |a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    nth.abs()
}

#[inline(always)]
pub(super) fn fit_plane_for_warp_padded(
    iso: &[f32],
    w_pad: usize,
    cols: usize,
    rows: usize,
    u: &[f32],
    v: &[f32],
) -> (f32, f32, f32) {
    let n = (cols * rows) as f32;
    let mut sum = 0.0f32;
    for j in 0..rows {
        let row = (j + 1) * w_pad;
        for i in 0..cols {
            sum += iso[row + (i + 1)];
        }
    }
    let mean_z = sum / n;

    let (mut su2, mut sv2, mut suv, mut suz, mut svz) = (0.0f32, 0.0, 0.0, 0.0, 0.0);

    for j in 0..rows {
        let vv = v[j] - 0.5;
        let row = (j + 1) * w_pad;
        for i in 0..cols {
            let uu = u[i] - 0.5;
            let z = iso[row + (i + 1)] - mean_z;
            su2 += uu * uu;
            sv2 += vv * vv;
            suv += uu * vv;
            suz += uu * z;
            svz += vv * z;
        }
    }

    let den = su2 * sv2 - suv * suv;
    if den.abs() < 1e-9 {
        return (0.0, 0.0, mean_z);
    }

    let a = (sv2 * suz - suv * svz) / den;
    let b = (su2 * svz - suv * suz) / den;
    let c = mean_z - a * 0.5 - b * 0.5;
    (a, b, c)
}
