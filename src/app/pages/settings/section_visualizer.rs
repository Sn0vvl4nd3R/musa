use egui::{self};
use crate::app::MusaApp;

pub(super) fn section_visualizer(app: &mut MusaApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Visualizer").default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("n_points");
            ui.add(egui::DragValue::new(&mut app.cfg.visualizer.n_points).clamp_range(16..=4096));
        });
        if ui.button("Apply visualizer size").clicked() {
            let n = app.cfg.visualizer.n_points.max(16).min(4096);
            if app.vis_draw.len() != n {
                app.vis_draw = vec![0.0; n];
            }
        }
    });
}
