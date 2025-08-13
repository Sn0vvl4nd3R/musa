use crate::app::MusaApp;
use crate::config::Config;
use egui::{
    self,
    Color32,
    RichText
};

fn c32(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}
fn to_rgb(c: Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}

pub(crate) fn ui_page_settings(app: &mut MusaApp, ui: &mut egui::Ui) {
    let cfg_path = Config::default_path();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(RichText::new("Settings").size(21.0).strong().color(app.header_color));
            ui.small(format!("Config file: {}", cfg_path.display()));
            ui.add_space(8.0);

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
                    ui.add(egui::DragValue::new(&mut app.cfg.anim.theme_ms).clamp_range(60..=5000).speed(10));
                });

                ui.add_space(6.0);
                if ui.button("Preview theme").clicked() {
                    let target_bg = crate::theme::make_gradient_stops(app.cfg.bg_stops_color32());
                    let target_accent = app.cfg.accent_color32();
                    crate::app::anim::begin_theme_anim(app, target_bg, target_accent);
                }
            });

            ui.add_space(8.0);

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

            ui.add_space(8.0);

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
                    if ui.add(egui::DragValue::new(&mut app.cfg.ui.volume.max_value).clamp_range(0.1..=5.0).speed(0.05)).changed() {
                        let maxv = app.cfg.ui.volume.max_value;
                        if app.player.volume > maxv {
                            app.player.set_volume(maxv);
                        }
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Bottom bar height");
                    ui.add(egui::DragValue::new(&mut app.cfg.ui.bottom_bar_h).clamp_range(40.0..=200.0).speed(0.5));
                });
            });

            ui.add_space(8.0);

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

            ui.add_space(8.0);

            egui::CollapsingHeader::new("Player").default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("initial_volume");
                    ui.add(egui::DragValue::new(&mut app.cfg.player.initial_volume).clamp_range(0.0..=app.cfg.ui.volume.max_value).speed(0.05));
                });
            });

            ui.add_space(8.0);

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

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

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

            if !app.status.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(&app.status).color(Color32::LIGHT_GREEN));
            }

            ui.add_space(app.cfg.ui.bottom_bar_h + app.cfg.ui.gap_small * 2.0);
        });
}
