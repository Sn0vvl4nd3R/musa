use egui;
use crate::app::MusaApp;
use super::cache::BgCache;

use super::math::{
    mirror01,
    smoothstep_inv
};

use crate::{
    app::anim::color::base_gradient,
    theme::{
        rgb_to_hsv,
        hsv_to_rgb,
    }
};

pub(super) fn render_mesh(
    app: &MusaApp,
    cache: &BgCache,
    rect: egui::Rect,
    h_acc: f32,
    light: (f32, f32, f32),
    music: f32,
    a_warp: f32,
    b_warp: f32,
    c_warp: f32,
    inv_du2: f32,
    inv_dv2: f32,
) -> egui::Mesh {
    let cols = cache.cols;
    let rows = cache.rows;
    let w_pad = cache.w_pad;

    let p = &app.cfg.anim.bg;
    let mut mesh = egui::Mesh::default();
    mesh.vertices.reserve_exact(cols * rows);

    for j in 0..rows {
        let v = cache.v[j];
        let y = egui::lerp(rect.top()..=rect.bottom(), v);
        let jpad = j + 1;
        let row = jpad * w_pad;

        for i in 0..cols {
            let u = cache.u[i];
            let x = egui::lerp(rect.left()..=rect.right(), u);
            let ip = i + 1;
            let idx = row + ip;

            let iso_c = cache.iso[idx];

            let ddu = (cache.iso[idx + 1] - cache.iso[idx - 1]) * inv_du2;
            let ddv = (cache.iso[idx + w_pad] - cache.iso[idx - w_pad]) * inv_dv2;

            let drift = a_warp * u + b_warp * v + c_warp;
            let iso_for_vrp = (iso_c - drift - cache.row_bias[j]) * 0.95;

            let warp_v = p.vis_gain * (p.warp_base + p.warp_music * music);
            let vv = mirror01(v + iso_for_vrp * warp_v);

            let base = base_gradient(vv, app);
            let (mut h, s, mut val) = rgb_to_hsv(base);

            let nxn = -ddu * p.depth_gain * (0.85 + 0.30 * music);
            let nyn = -ddv * p.depth_gain * (0.85 + 0.30 * music);
            let nzn = 1.0;
            let inv = 1.0 / (nxn * nxn + nyn * nyn + nzn * nzn).sqrt();
            let nxn = nxn * inv; let nyn = nyn * inv; let nzn = nzn * inv;

            let lambert = (nxn * light.0 + nyn * light.1 + nzn * light.2).clamp(0.0, 1.0);
            let mut rim = (1.0 - nzn).max(0.0); rim *= rim;

            let dx = u - 0.5;
            let dy = v - 0.5;
            let r = (dx * dx + dy * dy).sqrt();
            let center = smoothstep_inv(0.30, 0.85, r);
            let focus = center * p.focus + (1.0 - center) * (1.0 - p.focus);

            let vol  = p.vis_gain * (p.vol_base + p.vol_music * music) * focus;
            let lift = (lambert - 0.5) * p.lambert_gain + rim * p.rim_gain;
            val = (val + vol * lift).clamp(0.0, 1.0);

            let mut dh = h_acc - h;
            if dh > 180.0 {
                dh -= 360.0;
            }
            if dh < -180.0 {
                dh += 360.0;
            }
            h = (h + dh * p.hue_follow * focus).rem_euclid(360.0);

            let col = hsv_to_rgb(h, s, val);
            mesh.colored_vertex(egui::pos2(x, y), col);
        }
    }

    mesh
}
