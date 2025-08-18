use crate::app::MusaApp;
use egui::{self};

use super::helpers::{
    c32,
    to_rgb
};

pub(super) fn section_theme(app: &mut MusaApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Theme").default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Accent");
            let mut col = c32(app.cfg.theme.accent);
            if egui::color_picker::color_edit_button_srgba(
                ui, &mut col, egui::color_picker::Alpha::Opaque,
            ).changed() {
                app.cfg.theme.accent = to_rgb(col);
            }
        });

        ui.separator();
        ui.label("Background gradient stops (3 colors):");
        for i in 0..3 {
            ui.horizontal(|ui| {
                ui.label(format!("Stop {}", i + 1));
                let mut col = c32(app.cfg.theme.bg_stops[i]);
                if egui::color_picker::color_edit_button_srgba(
                    ui, &mut col, egui::color_picker::Alpha::Opaque,
                ).changed() {
                    app.cfg.theme.bg_stops[i] = to_rgb(col);
                }
            });
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Theme transition (ms)");
            ui.add(
                egui::DragValue::new(&mut app.cfg.anim.theme_ms)
                    .clamp_range(60..=5000)
                    .speed(10),
            );
        });

        ui.add_space(6.0);
        if ui.button("Preview theme").clicked() {
            let target_bg = crate::theme::make_gradient_stops(app.cfg.bg_stops_color32());
            let target_accent = app.cfg.accent_color32();
            crate::app::anim::begin_theme_anim(app, target_bg, target_accent);
        }
    });
}
