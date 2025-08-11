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
                ui.image((tex.id(), egui::vec2(size[0] as f32 * ratio, size[1] as f32 * ratio)));
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

                if ui.selectable_label(is_sel, txt).clicked() {
                    select_after = Some(i);
                }
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
