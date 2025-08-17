use eframe::egui::{
    self,
    Color32
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

pub fn accent_button(ui: &mut egui::Ui, label: &str, accent: Color32) -> egui::Response {
    let fill = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 36);
    let stroke = egui::Stroke::new(1.0, accent);
    ui.add(egui::Button::new(label).fill(fill).stroke(stroke))
}
