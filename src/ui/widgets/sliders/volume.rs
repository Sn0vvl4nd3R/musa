use crate::config::VolumeSliderCfg;
use eframe::egui::{
    self,
    Color32
};

pub fn volume_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    width: f32,
    accent: Color32,
    style: &VolumeSliderCfg,
) -> egui::Response {
    let h = style.height;
    let r = h * 0.5;

    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(width.max(style.width_min), h),
        egui::Sense::click_and_drag(),
    );
    ui.expand_to_include_rect(rect);

    let painter = ui.painter_at(rect);

    let bg = Color32::from_rgb(56, 56, 60);
    let border = Color32::from_rgb(92, 92, 96);

    painter.rect_filled(rect, r, bg);
    painter.rect_stroke(rect, r, egui::Stroke::new(1.0, border));

    let frac = (*value / style.max_value).clamp(0.0, 1.0);
    let w = rect.left() + rect.width() * frac;
    let filled = egui::Rect::from_min_max(rect.min, egui::pos2(w, rect.bottom()));
    painter.rect_filled(filled, r, accent);

    let knob_r = if resp.hovered() || resp.dragged() {
        style.knob_r_hover
    } else {
        style.knob_r
    };
    let center = egui::pos2(w, rect.center().y);
    painter.circle_filled(center, knob_r, accent);
    painter.circle_stroke(center, knob_r, egui::Stroke::new(1.0, border));

    if (resp.clicked() || resp.dragged()) && rect.width() > 0.0 {
        if let Some(p) = resp.interact_pointer_pos() {
            let frac = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            *value = style.max_value * frac;
        }
    }
    resp
}
