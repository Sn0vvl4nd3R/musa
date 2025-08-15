use egui::Color32;
use super::super::MusaApp;
use crate::theme::lerp_srgb;

#[inline]
pub(super) fn base_gradient(v: f32, app: &MusaApp) -> Color32 {
    if v <= 0.5 {
        lerp_srgb(app.bg_colors[0], app.bg_colors[1], v * 2.0)
    } else {
        lerp_srgb(app.bg_colors[1], app.bg_colors[2], (v - 0.5) * 2.0)
    }
}
