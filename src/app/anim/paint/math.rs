use crate::app::anim::flow::FlowParams;
use std::f32::consts::TAU;

const A0: f32 = 0.48;
const A1: f32 = 0.36;
const A2: f32 = 0.30;
const A3: f32 = 0.24;

const A_WEIGHTS: [f32; 4] = [A0, A1, A2, A3];
const INV_A_SUM: f32 = 1.0 / (A0 + A1 + A2 + A3);
const CENTER: f32 = 0.5;

#[inline(always)]
pub(super) fn smoothstep_inv(edge0: f32, edge1: f32, x: f32) -> f32 {
    let denom = edge1 - edge0;

    if denom.abs() <= f32::EPSILON {
        return if x <= edge0 { 1.0 } else { 0.0 };
    }

    let t = ((x - edge0) / denom).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    1.0 - s
}

#[inline(always)]
pub(super) fn mirror01(x: f32) -> f32 {
    let y = x.rem_euclid(2.0);
    1.0 - (y - 1.0).abs()
}

#[inline(always)]
pub(super) fn precompute_wtph(p: &FlowParams, t: f32) -> [f32; 4] {
    let wt = t * TAU;
    let mut a = [0.0; 4];

    for i in 0..4 {
        a[i] = p.w[i].mul_add(wt, p.ph[i]);
    }

    a
}

#[inline(always)]
pub(super) fn flow_four_sines(uu: f32, vv: f32, p: &FlowParams, wtph: &[f32; 4]) -> f32 {
    let mut acc = 0.0f32;

    for i in 0..4 {
        let k_dot_uv = p.ky[i].mul_add(vv, p.kx[i] * uu);
        let phase = k_dot_uv.mul_add(TAU, wtph[i]);
        let s = phase.sin();

        acc = A_WEIGHTS[i].mul_add(s, acc);
    }

    (acc * INV_A_SUM).clamp(-1.0, 1.0)
}

#[inline(always)]
pub(super) fn p90_abs(mut v: Vec<f32>) -> f32 {
    let len = v.len();
    if len == 0 {
        return 0.0;
    }

    let mut k = (9 * len) / 10;
    if k >= len {
        k = len - 1;
    }

    let (_, nth, _) = v.select_nth_unstable_by(k, |a, b| a.total_cmp(b));

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
    debug_assert!(cols > 0 && rows > 0);
    debug_assert!(u.len() >= cols);
    debug_assert!(v.len() >= rows);

    let cols_f = cols as f32;
    let rows_f = rows as f32;
    let n = cols_f * rows_f;

    let (mut sum_u, mut sum_u2) = (0.0f32, 0.0f32);
    for i in 0..cols {
        let uu = u[i] - CENTER;
        sum_u += uu;
        sum_u2 += uu * uu;
    }

    let (mut sum_v, mut sum_v2) = (0.0f32, 0.0f32);
    for j in 0..rows {
        let vv = v[j] - CENTER;
        sum_v += vv;
        sum_v2 += vv * vv;
    }

    let su2 = rows_f * sum_u2;
    let sv2 = cols_f * sum_v2;
    let suv = sum_u * sum_v;

    let (mut sum_z, mut sum_uz, mut sum_vz) = (0.0f32, 0.0f32, 0.0f32);

    for j in 0..rows {
        let vv = v[j] - CENTER;
        let row_offset = (j + 1) * w_pad;

        for i in 0..cols {
            let uu = u[i] - CENTER;
            let z = iso[row_offset + (i + 1)];

            sum_z += z;
            sum_uz += uu * z;
            sum_vz += vv * z;
        }
    }

    let mean_z = sum_z / n;

    let suz = sum_uz - mean_z * (rows_f * sum_u);
    let svz = sum_vz - mean_z * (cols_f * sum_v);

    let den = su2 * sv2 - suv * suv;
    if den.abs() < 1e-9 {
        return (0.0, 0.0, mean_z);
    }

    let a = (sv2 * suz - suv * svz) / den;
    let b = (su2 * svz - suv * suz) / den;
    let c = mean_z - a * CENTER - b * CENTER;

    (a, b, c)
}
