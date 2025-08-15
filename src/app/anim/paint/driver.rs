use egui::{self};
use crate::app::MusaApp;

use super::{
    math::p90_abs,
    cache::BG_CACHE,
    field::fill_iso,
    render::render_mesh,
    surface::compute_plane_and_bias,
};

use crate::{
    theme::rgb_to_hsv,
    app::anim::{
        flow::make_params,
        time_rng::{
            rand01,
            time_now
        }
    }
};

pub fn paint_bg_gradient(ctx: &egui::Context, app: &MusaApp) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());

    let seed = (app.accent.r() as u32) * 73856093
        ^ (app.accent.g() as u32) * 19349663
        ^ (app.accent.b() as u32) * 83492791;

    let p_main = make_params(seed);
    let p_detail = make_params(seed ^ 0xDEAD_BEEF);

    let bg = &app.cfg.anim.bg;
    let t = if bg.enabled {
        time_now()
    } else {
        0.0
    };

    let raw_music = {
        let raw = app.player.vis_buffer().take_recent(2048);
        if raw.len() >= 256 {
            let samp: Vec<f32> = raw.iter().step_by(8).map(|x| x.abs()).collect();
            (p90_abs(samp) * 1.6).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };

    let music = {
        if bg.enabled {
            raw_music * bg.music_amount
        } else {
            0.0
        }
    };

    let nx = ((rect.width() / 20.0).ceil() as usize).clamp(90, 220);
    let ny = ((rect.height() / 20.0).ceil() as usize).clamp(90, 220);
    let du = 1.0 / nx as f32;
    let dv = 1.0 / ny as f32;
    let inv_du2 = 0.5 / du;
    let inv_dv2 = 0.5 / dv;

    BG_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        cache.ensure(nx, ny);

        let phi = 2.0 * std::f32::consts::PI * (0.03 * t + rand01(seed ^ 0xABCD));
        let (sphi, cphi) = phi.sin_cos();
        let light = (cphi * 0.7, sphi * 0.7, 0.72);

        let (h_acc, _, _) = rgb_to_hsv(app.accent);

        fill_iso(&mut cache, &p_main, &p_detail, t);
        cache.blur_iso(1);

        let (a_warp, b_warp, c_warp) = compute_plane_and_bias(&mut cache);

        let w_pad = cache.w_pad;
        let rows  = cache.rows;
        let cols  = cache.cols;
        super::cache::BgCache::fill_padding_for(&mut cache.iso, w_pad, rows, cols);

        let mut mesh = render_mesh(
            app, &cache, rect, h_acc, light, music, a_warp, b_warp, c_warp, inv_du2, inv_dv2
        );
        mesh.indices.extend_from_slice(&cache.indices);
        painter.add(egui::Shape::mesh(mesh));
    });
}
