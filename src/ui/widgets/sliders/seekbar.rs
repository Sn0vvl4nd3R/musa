use crate::config::SeekbarCfg;

use eframe::egui::{
    self,
    Color32
};

pub enum SeekbarEvent {
    Preview(f32),
    Commit(f32),
}

pub fn seekbar(
    ui: &mut egui::Ui,
    pos: f32,
    total: f32,
    width: f32,
    style: &SeekbarCfg,
    accent: Color32,
) -> Option<SeekbarEvent> {
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

    let cur_frac = if have_total {
        (pos / total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut vis_frac = cur_frac;

    if have_total {
        if let Some(p) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.is_pointer_button_down_on() {
                vis_frac = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            }
        }

        let w = rect.left() + rect.width() * vis_frac;
        let played_rect = egui::Rect::from_min_max(rect.min, egui::pos2(w, rect.bottom()));
        painter.rect_filled(played_rect, r, played);

        let hover = resp.hovered() || resp.dragged();
        let knob_r = if hover {
            style.knob_r_hover
        } else {
            style.knob_r
        };
        let center = egui::pos2(w, rect.center().y);
        painter.circle_filled(center, knob_r, accent);
        painter.circle_stroke(center, knob_r, egui::Stroke::new(1.0, border));

        if resp.drag_stopped() {
            return Some(SeekbarEvent::Commit(total * vis_frac));
        }
        if resp.clicked() && !resp.dragged() {
            let frac = if let Some(p) = resp.interact_pointer_pos() {
                ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0)
            } else {
                vis_frac
            };
            return Some(SeekbarEvent::Commit(total * frac));
        }
        if resp.dragged() {
            return Some(SeekbarEvent::Preview(total * vis_frac));
        }
    } else {
        painter.rect_filled(rect, r, played);
    }
    None
}
