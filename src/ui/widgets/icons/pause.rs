use eframe::egui::{
    self,
    Color32
};

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
