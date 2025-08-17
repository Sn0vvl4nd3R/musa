use eframe::egui::{
    self,
    Color32
};

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
