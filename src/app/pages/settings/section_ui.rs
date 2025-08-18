use crate::app::MusaApp;

use egui::{
    self,
    RichText
};

pub(super) fn section_ui(app: &mut MusaApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("UI").default_open(true).show(ui, |ui| {
        ui.label(RichText::new("Gaps").strong());
        ui.horizontal(|ui| {
            ui.label("gap");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.gap).clamp_range(0.0..=32.0).speed(0.2));
            ui.label("gap_small");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.gap_small).clamp_range(0.0..=32.0).speed(0.2));
        });

        ui.separator();
        ui.label(RichText::new("Seekbar").strong());
        ui.horizontal(|ui| {
            ui.label("height");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.seek.height).clamp_range(6.0..=24.0).speed(0.2));
            ui.label("knob_r");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.seek.knob_r).clamp_range(2.0..=20.0).speed(0.2));
            ui.label("knob_r_hover");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.seek.knob_r_hover).clamp_range(2.0..=24.0).speed(0.2));
        });

        ui.separator();
        ui.label(RichText::new("Volume slider").strong());
        ui.horizontal(|ui| {
            ui.label("height");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.volume.height).clamp_range(6.0..=24.0).speed(0.2));
            ui.label("knob_r");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.volume.knob_r).clamp_range(1.0..=16.0).speed(0.2));
            ui.label("knob_r_hover");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.volume.knob_r_hover).clamp_range(1.0..=16.0).speed(0.2));
            ui.label("width_min");
            ui.add(egui::DragValue::new(&mut app.cfg.ui.volume.width_min).clamp_range(40.0..=400.0));
        });
        ui.horizontal(|ui| {
            ui.label("max_value");
            if ui
                .add(
                    egui::DragValue::new(&mut app.cfg.ui.volume.max_value)
                        .clamp_range(0.1..=5.0)
                        .speed(0.05),
                )
                .changed()
            {
                let maxv = app.cfg.ui.volume.max_value;
                if app.player.volume > maxv {
                    app.player.set_volume(maxv);
                }
            }
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Bottom bar height");
            ui.add(
                egui::DragValue::new(&mut app.cfg.ui.bottom_bar_h)
                    .clamp_range(40.0..=200.0)
                    .speed(0.5),
            );
        });
    });
}
