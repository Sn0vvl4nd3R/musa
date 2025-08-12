use super::*;
use egui::{
    Color32,
    RichText
};

use crate::{
    app::cover::update_cover_from_current_track,
    ui::widgets::{
        volume_slider,
        draw_icon_repeat
    }
};

pub(super) fn bottom_controls(app: &mut super::MusaApp, ui: &mut egui::Ui) {
    let total = app.player.current_total_secs();
    let have_total = total.is_finite() && total > 0.0;
    let mut pos = app.player.current_pos().as_secs_f32();
    if have_total && pos > total {
        pos = total;
    }

    let time_w: f32 = 54.0;
    let gap: f32 = 8.0;
    let row1_h: f32 = 22.0;
    let row1_w = ui.available_width();

    ui.allocate_ui_with_layout(
        egui::vec2(row1_w, row1_h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add_sized([time_w, row1_h], egui::Label::new(
                RichText::new(crate::util::seconds_to_mmss(pos))
                    .monospace().color(Color32::from_rgb(240,240,246))
            ));

            ui.add_space(gap);

            let seek_w = (row1_w - time_w * 2.0 - gap * 2.0).max(80.0);
            if let Some(new_secs) = seekbar(ui, pos, total, seek_w, 14.0, app.accent) {
                if let Err(e) = app.player.seek_to_secs(new_secs) {
                    app.status = format!("Seek error: {e}");
                }
            }

            ui.add_space(gap);

            ui.add_sized([time_w, row1_h], egui::Label::new(
                RichText::new(crate::util::seconds_to_mmss(if have_total {
                    total
                } else {
                    f32::NAN
                })).monospace().color(Color32::from_rgb(240,240,246))
            ));
        },
    );

    ui.add_space(6.0);

    let prev_d: f32 = 40.0;
    let play_d: f32 = 48.0;
    let next_d: f32 = 40.0;
    let rep_d: f32 = 36.0;
    let row2_h = play_d.max(28.0);

    let center_block_w = prev_d + play_d + next_d + rep_d + gap * 3.0;

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), row2_h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.columns(3, |cols| {
                cols[0].allocate_ui(egui::vec2(0.0, 0.0), |_ui| {});

                cols[1].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let pad = ((ui.available_width() - center_block_w) * 0.5).max(0.0);
                    ui.add_space(pad);

                    let prev_resp = icon_button_circle(ui, prev_d, "Previous track", |p, r, c| draw_icon_prev(p, r, c));
                    if prev_resp.clicked() {
                        if let Err(e) = app.player.prev() {
                            app.status = format!("Prev error: {e}");
                        } else {
                            update_cover_from_current_track(app);
                        }
                    }

                    ui.add_space(gap);

                    let label_tt = if app.player.is_playing() {
                        "Pause"
                    } else {
                        "Play"
                    };
                    let play_resp = icon_button_circle(ui, play_d, label_tt, |p, r, c| {
                        if app.player.is_playing() {
                            draw_icon_pause(p, r, c)
                        } else {
                            draw_icon_play(p, r, c)
                        }
                    });
                    if play_resp.clicked() {
                        app.player.toggle_pause();
                    }

                    ui.add_space(gap);

                    let next_resp = icon_button_circle(ui, next_d, "Next track", |p, r, c| draw_icon_next(p, r, c));
                    if next_resp.clicked() {
                        if let Err(e) = app.player.next() {
                            app.status = format!("Next error: {e}");
                        } else {
                            update_cover_from_current_track(app);
                        }
                    }

                    ui.add_space(gap);

                    let rep_tip = if app.player.repeat_one {
                        "Repeat this track: ON"
                    } else {
                        "Repeat this track: OFF"
                    };
                    let rep_resp = icon_button_circle(ui, rep_d, rep_tip, |p, r, c| {
                        let col = if app.player.repeat_one {
                            app.accent
                        } else {
                            c
                        };
                        draw_icon_repeat(p, r, col, app.player.repeat_one)
                    });
                    if rep_resp.clicked() {
                        app.player.toggle_repeat_one();
                    }

                    ui.add_space(pad);
                });

                cols[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(80.0);

                    let mut vol = app.player.volume;
                    let vol_w = (ui.available_width() * 0.18).clamp(90.0, 140.0);
                    let _ = volume_slider(ui, &mut vol, vol_w, app.accent);

                    if (vol - app.player.volume).abs() > f32::EPSILON {
                        app.player.set_volume(vol);
                    }
                });
            });
        },
    );

    if !app.status.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(&app.status).color(Color32::LIGHT_RED));
    }
}
