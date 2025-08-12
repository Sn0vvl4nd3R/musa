use eframe::egui::{
    self,
    Color32,
};

pub fn icon_button_circle(
    ui: &mut egui::Ui,
    diameter: f32,
    tooltip: &str,
    draw_icon: impl FnOnce(&egui::Painter, egui::Rect, Color32),
) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::click());
    let painter = ui.painter_at(rect);

    let visuals = ui.style().visuals.clone();
    let w_inactive = &visuals.widgets.inactive;
    let w_hovered = &visuals.widgets.hovered;
    let w_active = &visuals.widgets.active;

    let (bg_fill, bg_stroke, fg) = if resp.is_pointer_button_down_on() {
        (w_active.bg_fill, w_active.bg_stroke, w_active.fg_stroke.color)
    } else if resp.hovered() {
        (w_hovered.bg_fill, w_hovered.bg_stroke, w_hovered.fg_stroke.color)
    } else {
        (w_inactive.bg_fill, w_inactive.bg_stroke, w_inactive.fg_stroke.color)
    };

    let center = rect.center();
    let radius = diameter * 0.5;
    painter.circle_filled(center, radius, bg_fill);
    painter.circle_stroke(center, radius, egui::Stroke::new(1.0, Color32::WHITE));
    painter.circle_stroke(center, radius, bg_stroke);

    let icon_rect = rect.shrink(diameter * 0.28);
    draw_icon(&painter, icon_rect, fg);

    resp.on_hover_text(tooltip)
}

pub fn draw_icon_play(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let left = r.left();
    let right = r.right();
    let top = r.top();
    let bot = r.bottom();
    let cx = r.center().x;
    let pad_l = (cx - left) * 0.15;
    let p1 = egui::pos2(left + pad_l, top);
    let p2 = egui::pos2(right, r.center().y);
    let p3 = egui::pos2(left + pad_l, bot);
    p.add(egui::Shape::convex_polygon(vec![p1, p2, p3], color, egui::Stroke::new(0.0, color)));
}
pub fn draw_icon_pause(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let t = r.width() * 0.22;
    let gap = t * 0.45;
    let total = t * 2.0 + gap;
    let x0 = r.center().x - total * 0.5;
    let rect1 = egui::Rect::from_min_max(egui::pos2(x0, r.top()), egui::pos2(x0 + t, r.bottom()));
    let rect2 = egui::Rect::from_min_max(egui::pos2(x0 + t + gap, r.top()), egui::pos2(x0 + t + gap + t, r.bottom()));
    p.rect_filled(rect1, 1.0, color);
    p.rect_filled(rect2, 1.0, color);
}
pub fn draw_icon_next(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let rr = r.shrink(r.width() * 0.05);
    let tw = rr.width() * 0.30;
    let gap = tw * 0.30;
    let total = tw * 2.0 + gap;
    let x_left = rr.center().x - total * 0.5;
    let y_top = rr.top();
    let y_mid = rr.center().y;
    let y_bot = rr.bottom();
    let tri1 = vec![
        egui::pos2(x_left, y_top),
        egui::pos2(x_left + tw, y_mid),
        egui::pos2(x_left, y_bot),
    ];
    let x2 = x_left + tw + gap;
    let tri2 = vec![
        egui::pos2(x2, y_top),
        egui::pos2(x2 + tw, y_mid),
        egui::pos2(x2, y_bot),
    ];
    p.add(egui::Shape::convex_polygon(tri1, color, egui::Stroke::new(0.0, color)));
    p.add(egui::Shape::convex_polygon(tri2, color, egui::Stroke::new(0.0, color)));
}
pub fn draw_icon_prev(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let rr = r.shrink(r.width() * 0.05);
    let tw = rr.width() * 0.30;
    let gap = tw * 0.30;
    let total = tw * 2.0 + gap;
    let x_right = rr.center().x + total * 0.5;
    let y_top = rr.top();
    let y_mid = rr.center().y;
    let y_bot = rr.bottom();
    let tri1 = vec![
        egui::pos2(x_right, y_top),
        egui::pos2(x_right - tw, y_mid),
        egui::pos2(x_right, y_bot),
    ];
    let x2 = x_right - tw - gap;
    let tri2 = vec![
        egui::pos2(x2, y_top),
        egui::pos2(x2 - tw, y_mid),
        egui::pos2(x2, y_bot),
    ];
    p.add(egui::Shape::convex_polygon(tri1, color, egui::Stroke::new(0.0, color)));
    p.add(egui::Shape::convex_polygon(tri2, color, egui::Stroke::new(0.0, color)));
}
pub fn draw_icon_repeat(p: &egui::Painter, r: egui::Rect, color: Color32, one: bool) {
    let s = r.width().min(r.height());
    let center = r.center();

    let radius = s * 0.44;
    let sw = (s * 0.11).clamp(1.4, 2.6);
    let stroke = egui::Stroke::new(sw, color);

    let draw_arc = |a0: f32, a1: f32| {
        let steps = 28;
        let mut pts = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = a0 + (a1 - a0) * (i as f32) / (steps as f32);
            pts.push(egui::pos2(center.x + radius * t.cos(), center.y + radius * t.sin()));
        }
        p.add(egui::Shape::line(pts, stroke));
    };

    let tau = std::f32::consts::TAU;

    let a0 = 0.32 * tau;
    let a1 = 0.90 * tau;
    let a2 = 1.40 * tau;
    let a3 = 1.98 * tau;
    draw_arc(a0, a1);
    draw_arc(a2, a3);

    let len = (s * 0.28).max(2.0);
    let wid = (s * 0.18).max(1.5);
    let delta = 0.015 * tau;

    let arrow = |ang: f32| {
        let a = ang;
        let (sa, ca) = a.sin_cos();
        let tip_r = radius + sw * 0.55;
        let tip = egui::pos2(center.x + tip_r * ca, center.y + tip_r * sa);

        let tx = -sa;
        let ty = ca;
        let nx = ca;
        let ny = sa;

        let base = egui::pos2(tip.x - tx * len, tip.y - ty * len);
        let b1 = egui::pos2(base.x + nx * wid, base.y + ny * wid);
        let b2 = egui::pos2(base.x - nx * wid, base.y - ny * wid);

        p.add(egui::Shape::convex_polygon(vec![tip, b1, b2], color, egui::Stroke::NONE));
    };

    arrow(a1 + delta);
    arrow(a2 - delta);

    if one {
        let font = egui::FontId::proportional((s * 0.55).clamp(9.0, 22.0));
        p.text(center, egui::Align2::CENTER_CENTER, "1", font, color);
    }
}

pub fn seekbar(
    ui: &mut egui::Ui,
    pos: f32,
    total: f32,
    width: f32,
    height: f32,
    accent: Color32,
) -> Option<f32> {
    let have_total = total.is_finite() && total > 0.0;
    let sense = if have_total {
        egui::Sense::click_and_drag()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width.max(60.0), height), sense);
    ui.expand_to_include_rect(rect);

    let painter = ui.painter_at(rect);
    let bg = Color32::from_rgb(56, 56, 60);
    let border = Color32::from_rgb(92, 92, 96);
    let played = if have_total {
        accent
    } else {
        Color32::from_gray(110)
    };

    painter.rect_filled(rect, 6.0, bg);
    painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, border));

    let frac = if have_total {
        (pos / total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let w = rect.left() + rect.width() * frac;
    let played_rect = egui::Rect::from_min_max(rect.min, egui::pos2(w, rect.bottom()));
    painter.rect_filled(played_rect, 6.0, played);

    if have_total {
        let hover = resp.hovered() || resp.dragged();
        let r = if hover {
            7.5
        } else {
            6.5
        };
        let center = egui::pos2(w, rect.center().y);
        painter.circle_filled(center, r, accent);
        painter.circle_stroke(center, r, egui::Stroke::new(1.0, border));

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

pub fn accent_button(ui: &mut egui::Ui, label: &str, accent: Color32) -> egui::Response {
    let fill = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 36);
    let stroke = egui::Stroke::new(1.0, accent);
    ui.add(egui::Button::new(label).fill(fill).stroke(stroke))
}
