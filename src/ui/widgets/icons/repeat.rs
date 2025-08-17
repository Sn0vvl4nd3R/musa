use eframe::egui::{
    self,
    Color32
};

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
