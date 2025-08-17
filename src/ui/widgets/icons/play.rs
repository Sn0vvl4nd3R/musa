use eframe::egui::{
    self,
    Color32
};

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
