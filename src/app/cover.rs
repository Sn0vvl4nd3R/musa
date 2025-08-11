use egui::Color32;
use super::MusaApp;
use image::GenericImageView;
use std::{
    thread,
    sync::mpsc,
};
use crate::theme::{
    extract_palette,
    make_gradient_stops,
    accent_from_palette,
};

pub(super) fn update_cover_from_current_track(app: &mut MusaApp) {
    if let Some(dir) = app.player.current_album_dir() {
        app.cover_path = crate::cover::find_cover_image(&dir);
        app.cover_id_path = None;
        request_cover_load(app);
    } else {
        app.cover_path = None;
        app.cover_tex = None;
        app.cover_id_path = None;
        app.cover_rx = None;
    }
}

pub(super) fn request_cover_load(app: &mut MusaApp) {
    if let Some(path) = &app.cover_path {
        let key = path.display().to_string();
        if app.cover_id_path.as_deref() == Some(&key) || app.cover_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        app.cover_rx = Some(rx);
        let p = path.clone();
        let k = key.clone();
        thread::spawn(move || {
            let res: anyhow::Result<(usize, usize, Vec<u8>, String, [Color32; 3])> = (|| {
                let img = image::open(&p)?;
                let (w, h) = img.dimensions();
                let pal = extract_palette(&img, 3);
                Ok((w as usize, h as usize, img.to_rgba8().into_raw(), k, pal))
            })();
            let _ = tx.send(res);
        });
    }
}

pub(super) fn poll_cover_result(app: &mut MusaApp, ctx: &egui::Context) {
    if let Some(rx) = &app.cover_rx {
        match rx.try_recv() {
            Ok(Ok((w, h, rgba, key, pal))) => {
                let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                let tex = ctx.load_texture("cover_art", img, egui::TextureOptions::LINEAR);
                app.cover_tex = Some(tex);
                app.cover_id_path = Some(key);

                let target_bg = make_gradient_stops(pal);
                let target_accent = accent_from_palette(pal);
                super::anim::begin_theme_anim(app, target_bg, target_accent);

                app.cover_rx = None;
            }
            Ok(Err(e)) => {
                app.status = format!("Cover load error: {e}");
                app.cover_tex = None;
                app.cover_id_path = None;
                app.cover_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                app.cover_tex = None;
                app.cover_id_path = None;
                app.cover_rx = None;
            }
        }
    }
}
