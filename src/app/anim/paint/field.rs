use super::cache::BgCache;
use crate::app::anim::flow::FlowParams;

use super::math::{
    precompute_wtph,
    flow_four_sines,
};

pub(super) fn fill_iso(
    cache: &mut BgCache,
    p_main: &FlowParams,
    p_detail: &FlowParams,
    t: f32,
) {
    let rows = cache.rows;
    let cols = cache.cols;
    let w_pad = cache.w_pad;

    let wt_main = precompute_wtph(p_main, t);
    let t2 = t * 0.97;
    let wt_detail = precompute_wtph(p_detail, t2);

    let (s06, c06) = (0.6 * t).sin_cos();
    let (s05, c05) = (0.5 * t).sin_cos();
    let (s04, c04) = (0.4 * t).sin_cos();
    let (s03, c03) = (0.3 * t).sin_cos();

    for j in 0..rows {
        let v = cache.v[j];
        let jpad = j + 1;
        let row = jpad * w_pad;

        let v2 = v * 1.03 + 0.047;

        let cos_07v_t = cache.cos_07v[j] * c04 + cache.sin_07v[j] * s04;
        let sin_08v_t = cache.sin_08v[j] * c03 + cache.cos_08v[j] * s03;

        for i in 0..cols {
            let u = cache.u[i];
            let idx = row + (i + 1);

            let sin_09u_t = cache.sin_09u[i] * c06 + cache.cos_09u[i] * s06;
            let cos_06u_t = cache.cos_06u[i] * c05 + cache.sin_06u[i] * s05;

            let du1 = (sin_09u_t * cos_07v_t) * p_main.warp;
            let dv1 = (cos_06u_t * sin_08v_t) * p_main.warp;
            let uu1 = u + du1;
            let vv1 = v + dv1;

            let f1 = flow_four_sines(uu1, vv1, p_main, &wt_main);

            let u2 = u * 1.07 + 0.031;
            let v2_local = v2;
            let du2 = ( (0.9*u2 + 0.6*t2).sin() * (0.7*v2_local - 0.4*t2).cos() ) * p_detail.warp;
            let dv2 = ( (0.6*u2 - 0.5*t2).cos() * (0.8*v2_local + 0.3*t2).sin() ) * p_detail.warp;
            let uu2 = u2 + du2;
            let vv2 = v2_local + dv2;

            let f2 = flow_four_sines(uu2, vv2, p_detail, &wt_detail);

            cache.iso[idx] = (0.6f32 * f1 + 0.4f32 * f2).clamp(-1.0f32, 1.0f32);
        }
    }
}
