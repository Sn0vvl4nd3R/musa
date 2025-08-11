use egui::Color32;
use super::super::MusaApp;
use crate::theme::lerp_srgb;

#[inline]
pub(super) fn rgb_to_hsv(c: Color32) -> (f32, f32, f32) {
    let r = c.r() as f32 / 255.0;
    let g = c.g() as f32 / 255.0;
    let b = c.b() as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;

    let h = if d < 1e-6 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };

    let s = if max <= 0.0 {
        0.0
    } else {
        d / max
    };
    let h = if h < 0.0 {
        h + 360.0
    } else {
        h
    };
    (h, s, max)
}

#[inline]
pub(super) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color32::from_rgb(
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

#[inline]
pub(super) fn base_gradient(v: f32, app: &MusaApp) -> Color32 {
    if v <= 0.5 {
        lerp_srgb(app.bg_colors[0], app.bg_colors[1], v * 2.0)
    } else {
        lerp_srgb(app.bg_colors[1], app.bg_colors[2], (v - 0.5) * 2.0)
    }
}
