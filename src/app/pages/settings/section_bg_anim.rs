use crate::app::MusaApp;

use egui::{
    self,
    Color32,
    RichText
};

pub(super) fn section_bg_anim(app: &mut MusaApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Background animation").default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            let mut en = app.cfg.anim.bg.enabled;
            if ui.checkbox(&mut en, "Animated background (enabled)").changed() {
                app.cfg.anim.bg.enabled = en;
            }
            ui.label(RichText::new("Toggle to make theme static").italics().color(Color32::from_gray(170)));
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Music amount");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.music_amount).clamp_range(0.0..=1.0).speed(0.01));
        });

        ui.separator();
        ui.label(RichText::new("Shading / geometry").strong());
        ui.horizontal(|ui| {
            ui.label("vis_gain");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.vis_gain).clamp_range(0.1..=3.0).speed(0.01));
            ui.label("depth_gain");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.depth_gain).clamp_range(0.1..=3.0).speed(0.01));
            ui.label("focus");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.focus).clamp_range(0.0..=1.0).speed(0.01));
        });

        ui.horizontal(|ui| {
            ui.label("warp_base");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.warp_base).clamp_range(0.0..=0.2).speed(0.001));
            ui.label("warp_music");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.warp_music).clamp_range(0.0..=0.2).speed(0.001));
        });

        ui.horizontal(|ui| {
            ui.label("lambert_gain");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.lambert_gain).clamp_range(0.0..=2.0).speed(0.01));
            ui.label("rim_gain");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.rim_gain).clamp_range(0.0..=1.0).speed(0.01));
        });

        ui.horizontal(|ui| {
            ui.label("vol_base");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.vol_base).clamp_range(0.0..=1.0).speed(0.01));
            ui.label("vol_music");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.vol_music).clamp_range(0.0..=1.0).speed(0.01));
        });

        ui.horizontal(|ui| {
            ui.label("hue_follow");
            ui.add(egui::DragValue::new(&mut app.cfg.anim.bg.hue_follow).clamp_range(0.0..=0.2).speed(0.001));
        });
    });
}
