use egui::{self};
use crate::app::MusaApp;

pub(super) fn section_player(app: &mut MusaApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Player").default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("initial_volume");
            ui.add(
                egui::DragValue::new(&mut app.cfg.player.initial_volume)
                    .clamp_range(0.0..=app.cfg.ui.volume.max_value)
                    .speed(0.05),
            );
        });
    });
}
