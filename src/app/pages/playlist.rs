use super::*;
use crate::app::cover::update_cover_from_current_track;
use egui::{
    RichText,
    Color32,
};

pub(crate) fn ui_page_playlist(app: &mut super::MusaApp, ui: &mut egui::Ui) {
    ui.columns(2, |cols| {
        cols[0].with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let avail = ui.available_size();
            let reserve = 60.0;
            let side_from_height = (avail.y - reserve).max(100.0);
            let side = side_from_height.min(avail.x).min(360.0);
            if let Some(tex) = &app.cover_tex {
                let size = tex.size();
                let ratio = (side / size[0] as f32).min(side / size[1] as f32).min(1.0);
                let img_size = egui::vec2(size[0] as f32 * ratio, size[1] as f32 * ratio);
                
                let r = egui::Rounding::same((img_size.x.min(img_size.y) * 0.06).clamp(6.0, 16.0));
                let resp = ui.add(egui::Image::new((tex.id(), img_size)).rounding(r));
                ui.painter().rect_stroke(
                    resp.rect, r, egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0,0,0,90))
                );
            } else {
                ui.label(RichText::new("No cover").color(Color32::GRAY));
            }
            ui.add_space(6.0);
            if let Some(t) = app.player.current_track() {
                if !t.album.is_empty() {
                    ui.label(RichText::new(&t.album).size(16.0).color(app.title_color));
                }
            }
        });

        cols[1].label(RichText::new("Playlist").size(21.0).strong().color(app.header_color));
        let mut select_after: Option<usize> = None;
        let n = app.player.playlist.len();
        let on_accent = color::best_on(app.accent);

        egui::ScrollArea::vertical().show(&mut cols[1], |ui| {
            for i in 0..n {
                let t = &app.player.playlist[i];
                let is_sel = i == app.player.index;

                let mut txt = RichText::new(format!("{:>3}. {}", i + 1, t.display_line()));
                if is_sel {
                    txt = txt.color(on_accent).strong();
                }

                let resp = ui.selectable_label(is_sel, txt);
                if resp.clicked() {
                    select_after = Some(i);
                }

                let dur_txt = t.duration
                    .map(|d| crate::util::seconds_to_mmss(d.as_secs_f32()))
                    .unwrap_or_else(|| "--:--".to_string());

                let color = if is_sel {
                    on_accent
                } else {
                    ui.style().visuals.widgets.inactive.fg_stroke.color
                };
                let font = egui::FontId::proportional(13.0);
                let right_pad = 8.0;
                let pos = egui::pos2(resp.rect.right() - right_pad, resp.rect.center().y);

                ui.painter().text(pos, egui::Align2::RIGHT_CENTER, dur_txt, font, color);
            }
        });
        if let Some(i) = select_after {
            app.player.index = i;
            if let Err(e) = app.player.play_current() {
                app.status = format!("Playback error: {e}");
            } else {
                update_cover_from_current_track(app);
            }
        }
    });
}
