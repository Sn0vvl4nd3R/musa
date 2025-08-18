use egui::{self};

use crate::{
    app::MusaApp,
    config::Config
};

pub(super) fn section_footer(app: &mut MusaApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("Preview theme").clicked() {
            let target_bg = crate::theme::make_gradient_stops(app.cfg.bg_stops_color32());
            let target_accent = app.cfg.accent_color32();
            crate::app::anim::begin_theme_anim(app, target_bg, target_accent);
        }
        if ui.button("Save").clicked() {
            match app.cfg.save_to_default() {
                Ok(()) => app.status = "Config saved".into(),
                Err(e) => app.status = format!("Save error: {e}"),
            }
        }
        if ui.button("Reload from disk").clicked() {
            match Config::load_or_create() {
                Ok(new_cfg) => {
                    app.cfg = new_cfg;
                    let target_bg = crate::theme::make_gradient_stops(app.cfg.bg_stops_color32());
                    let target_accent = app.cfg.accent_color32();
                    crate::app::anim::begin_theme_anim(app, target_bg, target_accent);
                    let n = app.cfg.visualizer.n_points.max(16).min(4096);
                    if app.vis_draw.len() != n {
                        app.vis_draw = vec![0.0; n];
                    }
                    app.status = "Config reloaded".into();
                }
                Err(e) => app.status = format!("Reload error: {e}"),
            }
        }
        if ui.button("Reset to defaults").clicked() {
            app.cfg = Config::default();
            let target_bg = crate::theme::make_gradient_stops(app.cfg.bg_stops_color32());
            let target_accent = app.cfg.accent_color32();
            crate::app::anim::begin_theme_anim(app, target_bg, target_accent);
            let n = app.cfg.visualizer.n_points.max(16).min(4096);
            app.vis_draw = vec![0.0; n];
            app.status = "Config reset to defaults (not saved)".into();
        }
    });
}
