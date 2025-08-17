use crate::config::SeekbarCfg;

use eframe::egui::{
    self,
    Color32
};

pub fn seekbar(
    ui: &mut egui::Ui,
    pos: f32,
    total: f32,
    width: f32,
    style: &SeekbarCfg,
    accent: Color32,
) -> Option<f32> {
    let have_total = total.is_finite() && total > 0.0;
    let sense = if have_total {
        egui::Sense::click_and_drag()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width.max(60.0), style.height), sense);
    ui.expand_to_include_rect(rect);

    let painter = ui.painter_at(rect);
    let bg = Color32::from_rgb(56, 56, 60);
    let border = Color32::from_rgb(92, 92, 96);
    let played = if have_total {
        accent
    } else {
        Color32::from_gray(110)
    };

    let r = style.height * 0.5;

    painter.rect_filled(rect, r, bg);
    painter.rect_stroke(rect, r, egui::Stroke::new(1.0, border));

    let frac = if have_total {
        (pos / total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let w = rect.left() + rect.width() * frac;
    let played_rect = egui::Rect::from_min_max(rect.min, egui::pos2(w, rect.bottom()));
    painter.rect_filled(played_rect, r, played);

    if have_total {
        let hover = resp.hovered() || resp.dragged();
        let knob_r = if hover {
            style.knob_r_hover
        } else {
            style.knob_r
        };
        let center = egui::pos2(w, rect.center().y);
        painter.circle_filled(center, knob_r, accent);
        painter.circle_stroke(center, knob_r, egui::Stroke::new(1.0, border));

        if resp.clicked() || resp.dragged() {
            if let Some(p) = resp.interact_pointer_pos() {
                let frac = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                let new_secs = total * frac;
                return Some(new_secs.max(0.0));
            }
        }
    }
    None
}
