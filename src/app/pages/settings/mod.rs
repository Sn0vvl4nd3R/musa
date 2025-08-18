use crate::{
    app::MusaApp,
    config::Config
};

use egui::{
    self,
    Color32,
    RichText
};

mod helpers;
mod section_theme;
mod section_bg_anim;
mod section_ui;
mod section_visualizer;
mod section_player;
mod section_window;
mod footer_actions;

pub(crate) fn ui_page_settings(app: &mut MusaApp, ui: &mut egui::Ui) {
    let cfg_path = Config::default_path();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(RichText::new("Settings").size(21.0).strong().color(app.header_color));
            ui.small(format!("Config file: {}", cfg_path.display()));
            ui.add_space(8.0);

            section_theme::section_theme(app, ui);

            ui.add_space(8.0);
            section_bg_anim::section_bg_anim(app, ui);

            ui.add_space(8.0);
            section_ui::section_ui(app, ui);

            ui.add_space(8.0);
            section_visualizer::section_visualizer(app, ui);

            ui.add_space(8.0);
            section_player::section_player(app, ui);

            ui.add_space(8.0);
            section_window::section_window(app, ui);

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            footer_actions::section_footer(app, ui);

            if !app.status.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(&app.status).color(Color32::LIGHT_GREEN));
            }

            ui.add_space(app.cfg.ui.bottom_bar_h + app.cfg.ui.gap_small * 2.0);
        });
}
