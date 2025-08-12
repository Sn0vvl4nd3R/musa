use crate::app::color::best_on;
use crate::app::visualizer::draw_visualizer_bg;
use egui::{
    Color32,
    RichText,
};

pub(crate) fn ui_page_player(app: &mut super::MusaApp, ui: &mut egui::Ui) {
    if let Err(e) = app.player.auto_advance_if_needed() {
        app.status = format!("Auto-advance error: {e}");
    }

    draw_visualizer_bg(app, ui, app.dt_sec);

    ui.vertical_centered(|ui| {
        let avail = ui.available_size();
        let reserve_under_image = 160.0;
        let max_side = 460.0;
        let side_from_height = (avail.y - reserve_under_image).max(100.0);
        let side = side_from_height.min(avail.x).min(max_side);

        if let Some(tex) = &app.cover_tex {
            let size = tex.size();
            let ratio = (side / size[0] as f32)
                .min(side / size[1] as f32)
                .min(1.0);
            let img_size = egui::vec2(size[0] as f32 * ratio, size[1] as f32 * ratio);
            let on = best_on(app.accent);
            let stroke_col = Color32::from_rgba_unmultiplied(on.r(), on.g(), on.b(), 40);

            let r = egui::Rounding::same((img_size.x.min(img_size.y) * 0.06).clamp(6.0, 18.0));
            let resp = ui.add(egui::Image::new((tex.id(), img_size)).rounding(r));

            ui.painter().rect_stroke(
                resp.rect, r, egui::Stroke::new(1.0, stroke_col)
            );
        } else {
            ui.label(RichText::new("No cover").color(Color32::GRAY));
        }

        ui.add_space(8.0);
        let (title, artist, album) = if let Some(t) = app.player.current_track() {
            t.display_now_playing()
        } else {
            ("—".into(), "".into(), "".into())
        };

        ui.label(RichText::new(title).strong().size(24.0).color(app.title_color));
        if !artist.is_empty() {
            ui.label(RichText::new(artist).size(18.0));
        }
        if !album.is_empty()  {
            ui.label(RichText::new(album).size(16.0).color(Color32::from_gray(210)));
        }
    });

    if !app.status.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(&app.status).color(Color32::LIGHT_RED));
    }
}
