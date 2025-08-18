use crate::app::MusaApp;

use egui::{
    self,
    Color32,
    RichText
};

pub(super) fn section_window(app: &mut MusaApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Window").default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("start_w / start_h");
            ui.add(egui::DragValue::new(&mut app.cfg.window.start_w).clamp_range(400.0..=4000.0));
            ui.add(egui::DragValue::new(&mut app.cfg.window.start_h).clamp_range(300.0..=3000.0));
        });
        ui.horizontal(|ui| {
            ui.label("min_w / min_h");
            ui.add(egui::DragValue::new(&mut app.cfg.window.min_w).clamp_range(200.0..=4000.0));
            ui.add(egui::DragValue::new(&mut app.cfg.window.min_h).clamp_range(200.0..=4000.0));
        });
        ui.horizontal(|ui| {
            let mut vs = app.cfg.window.vsync;
            if ui.checkbox(&mut vs, "vsync (требует перезапуск)").changed() {
                app.cfg.window.vsync = vs;
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Apply window size now").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    app.cfg.window.start_w, app.cfg.window.start_h
                )));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                    app.cfg.window.min_w, app.cfg.window.min_h
                )));
            }
            ui.label(RichText::new("VSync меняется только после перезапуска").italics().color(Color32::from_gray(180)));
        });
    });
}
