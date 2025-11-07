use std::f32::consts::PI;

use crate::app::MusaApp;
use egui::{Context, LayerId, Shape};

use super::{
    cache::BG_CACHE, field::fill_iso, math::p90_abs, render::render_mesh,
    surface::compute_plane_and_bias,
};

use crate::{
    app::anim::{
        flow::make_params,
        time_rng::{rand01, time_now},
    },
    theme::rgb_to_hsv,
};

const VIS_BUFFER_SIZE: usize = 2048;
const MIN_VIS_SAMPLES: usize = 256;
const AUDIO_STEP: usize = 8;
const MUSIC_GAIN: f32 = 1.6;

const GRID_SCALE: f32 = 20.0;
const GRID_MIN: usize = 90;
const GRID_MAX: usize = 220;

const DETAIL_SEED_XOR: u32 = 0xDEAD_BEEF;
const LIGHT_SEED_XOR: u32 = 0xABCD;

pub fn paint_bg_gradient(ctx: &Context, app: &MusaApp) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(LayerId::background());

    let accent = app.accent;
    let bg_cfg = &app.cfg.anim.bg;

    let seed = (accent.r() as u32) * 73856093
        ^ (accent.g() as u32) * 19349663
        ^ (accent.b() as u32) * 83492791;

    let p_main = make_params(seed);
    let p_detail = make_params(seed ^ DETAIL_SEED_XOR);

    let t = if bg_cfg.enabled { time_now() } else { 0.0 };

    let use_music = bg_cfg.enabled && bg_cfg.music_amount > 0.0;
    let music = if use_music {
        let raw = app.player.vis_buffer().take_recent(VIS_BUFFER_SIZE);

        if raw.len() >= MIN_VIS_SAMPLES {
            let samples_len = (raw.len() + AUDIO_STEP - 1) / AUDIO_STEP;
            let mut samp = Vec::with_capacity(samples_len);

            for &v in raw.iter().step_by(AUDIO_STEP) {
                samp.push(v.abs());
            }

            let energy = (p90_abs(samp) * MUSIC_GAIN).clamp(0.0, 1.0);
            energy * bg_cfg.music_amount
        } else {
            0.0
        }
    } else {
        0.0
    };

    let nx = ((rect.width() / GRID_SCALE).ceil() as usize).clamp(GRID_MIN, GRID_MAX);
    let ny = ((rect.height() / GRID_SCALE).ceil() as usize).clamp(GRID_MIN, GRID_MAX);

    let du = 1.0 / nx as f32;
    let dv = 1.0 / ny as f32;
    let inv_du2 = 0.5 / du;
    let inv_dv2 = 0.5 / dv;

    let (h_acc, _, _) = rgb_to_hsv(accent);

    let phi = 2.0 * PI * (0.03 * t + rand01(seed ^ LIGHT_SEED_XOR));
    let (sphi, cphi) = phi.sin_cos();
    let light = (cphi * 0.7, sphi * 0.7, 0.72);

    BG_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        cache.ensure(nx, ny);

        fill_iso(&mut cache, &p_main, &p_detail, t);
        cache.blur_iso(1);

        let (a_warp, b_warp, c_warp) = compute_plane_and_bias(&mut cache);

        let w_pad = cache.w_pad;
        let rows = cache.rows;
        let cols = cache.cols;
        super::cache::BgCache::fill_padding_for(&mut cache.iso, w_pad, rows, cols);

        let mut mesh = render_mesh(
            app, &cache, rect, h_acc, light, music, a_warp, b_warp, c_warp, inv_du2, inv_dv2,
        );

        mesh.indices.extend_from_slice(&cache.indices);

        painter.add(Shape::mesh(mesh));
    });
}
