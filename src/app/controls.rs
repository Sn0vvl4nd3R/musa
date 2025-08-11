use super::*;
use crate::app::cover::update_cover_from_current_track;
use egui::{
    Color32,
    RichText
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
    let row1_h = 22.0;

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), row1_h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add_sized([time_w, row1_h], egui::Label::new(
                RichText::new(seconds_to_mmss(pos)).monospace().color(Color32::from_rgb(240,240,246))
            ));
            ui.add_space(gap);
            let seek_w = (ui.available_width() - (time_w + gap)).max(80.0);
            if let Some(new_secs) = seekbar(ui, pos, total, seek_w, 14.0, app.accent) {
                if let Err(e) = app.player.seek_to_secs(new_secs) {
                    app.status = format!("Seek error: {e}");
                }
            }
            ui.add_space(gap);
            ui.add_sized([time_w, row1_h], egui::Label::new(
                RichText::new(seconds_to_mmss(if have_total {
                    total
                } else {
                    f32::NAN
                })).monospace().color(Color32::from_rgb(240,240,246)),
            ));
        },
    );

    ui.add_space(6.0);

    let prev_d: f32 = 40.0;
    let play_d: f32 = 48.0;
    let next_d: f32 = 40.0;
    let center_block_w = prev_d + play_d + next_d + gap * 2.0;
    let vol_w: f32 = ui.available_width().min(320.0).max(180.0);
    let row2_h = play_d.max(30.0);

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), row2_h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let w_all = ui.available_width();
            let left_pad = ((w_all - vol_w - center_block_w) / 2.0 - gap).max(0.0);
            ui.allocate_space(egui::vec2(left_pad, 0.0));

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

            let used = left_pad + center_block_w + gap;
            let remain = (w_all - used - vol_w).max(0.0);
            ui.allocate_space(egui::vec2(remain, 0.0));

            let mut vol = app.player.volume;
            let slider_resp = ui.add_sized(
                [vol_w, 18.0],
                egui::Slider::new(&mut vol, 0.0..=2.0)
                .show_value(false)
            );
            let slider_rect = slider_resp.rect;
            ui.painter().rect_stroke(slider_rect, 3.0, egui::Stroke::new(1.0, Color32::WHITE));
            if vol != app.player.volume {
                app.player.set_volume(vol);
            }
        },
    );

    if !app.status.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(&app.status).color(Color32::LIGHT_RED));
    }
}
