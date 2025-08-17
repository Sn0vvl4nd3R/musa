use eframe::egui::{
    self,
    Color32
};

pub fn draw_icon_shuffle(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let rr = r.shrink(r.width() * 0.18);
    let w = rr.width();
    let h = rr.height();
    let cy = rr.center().y;
    let x0 = rr.left();
    let x1 = rr.right();

    let stroke = egui::Stroke::new((w * 0.08).clamp(1.5, 2.2), color);

    let y_t0 = cy - h * 0.22;
    let y_t1 = cy + h * 0.18;
    let top = vec![
        egui::pos2(x0, y_t0),
        egui::pos2(x0 + w * 0.30, y_t0),
        egui::pos2(x0 + w * 0.55, y_t1),
        egui::pos2(x1 - w * 0.18, y_t1),
    ];
    p.add(egui::Shape::line(top, stroke));

    let y_b0 = cy + h * 0.22;
    let y_b1 = cy - h * 0.18;
    let bot = vec![
        egui::pos2(x0, y_b0),
        egui::pos2(x0 + w * 0.30, y_b0),
        egui::pos2(x0 + w * 0.55, y_b1),
        egui::pos2(x1 - w * 0.18, y_b1),
    ];
    p.add(egui::Shape::line(bot, stroke));

    let arrow = |y: f32| {
        let tip = egui::pos2(x1, y);
        let len = w * 0.16;
        let wid = h * 0.20;
        let base = egui::pos2(x1 - len, y);
        let b1 = egui::pos2(base.x, base.y - wid * 0.5);
        let b2 = egui::pos2(base.x, base.y + wid * 0.5);
        p.add(egui::Shape::convex_polygon(vec![tip, b1, b2], color, egui::Stroke::new(0.0, color)));
    };
    arrow(y_t1);
    arrow(y_b1);
}
