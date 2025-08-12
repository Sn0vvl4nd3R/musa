use super::cache::BgCache;
use super::math::fit_plane_for_warp_padded;

pub(super) fn compute_plane_and_bias(cache: &mut BgCache) -> (f32, f32, f32) {
    let cols = cache.cols;
    let rows = cache.rows;
    let w_pad = cache.w_pad;

    let (a_warp, b_warp, c_warp) =
        fit_plane_for_warp_padded(&cache.iso, w_pad, cols, rows, &cache.u, &cache.v);

    let drift_u_avg = 0.5 * a_warp;
    for j in 0..rows {
        let vj = cache.v[j];
        let row = (j + 1) * w_pad;
        let mut sum = 0.0f32;
        for i in 0..cols {
            sum += cache.iso[row + (i + 1)];
        }
        let mean = sum / (cols as f32);
        cache.row_bias[j] = mean - (drift_u_avg + b_warp * vj + c_warp);
    }

    (a_warp, b_warp, c_warp)
}
