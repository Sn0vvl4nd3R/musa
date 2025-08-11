use egui::{
    self,
    Color32
};

pub fn apply_visuals(ctx: &egui::Context, accent: Color32) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.override_text_color = Some(Color32::from_rgb(245, 245, 250));

    v.panel_fill = Color32::TRANSPARENT;
    v.window_stroke = egui::Stroke::NONE;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;

    v.widgets.inactive.bg_fill = Color32::from_rgb(22, 22, 24);
    v.widgets.hovered.bg_fill = Color32::from_rgb(34, 34, 38);
    v.widgets.active.bg_fill = Color32::from_rgb(44, 44, 48);

    v.widgets.inactive.fg_stroke.color = Color32::from_rgb(235,235,242);
    v.widgets.hovered.fg_stroke.color = Color32::WHITE;
    v.widgets.active.fg_stroke.color = Color32::WHITE;

    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0,0,0,160));
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0,0,0,130));

    v.selection.bg_fill = accent;
    v.selection.stroke.color = Color32::WHITE;

    ctx.set_style(style);
}
