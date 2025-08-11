use std::path::PathBuf;
use crate::track::Track;
use crate::app::scanner;
use crate::util::parse_track_meta;
use crate::ui::widgets::accent_button;
use egui::{
    Color32,
    RichText,
};

pub(crate) fn ui_page_browser(app: &mut super::MusaApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if accent_button(ui,"Home", app.accent).clicked() {
            scanner::go_home(app);
        }
        if accent_button(ui, "Up", app.accent).clicked() {
            scanner::go_up(app);
        }
        let mut goto = false;
        ui.label(RichText::new("Path:").strong().color(app.accent));
        let edit = ui.text_edit_singleline(&mut app.current_dir_text);
        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            goto = true;
        }
        if accent_button(ui, "Go", app.accent).clicked() {
            goto = true;
        }
        if goto {
            let p = PathBuf::from(app.current_dir_text.clone());
            if p.is_dir() {
                scanner::navigate_to(app, p);
                app.status.clear();
            } else {
                app.status = "Path does not exist or is not a directory".into();
            }
        }
        if accent_button(ui, "Scan this folder", app.accent).clicked() {
            scanner::start_scan_current(app);
        }
    });

    if !app.status.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(&app.status).color(Color32::LIGHT_RED));
    }

    ui.add_space(8.0);

    let mut dir_to_open: Option<PathBuf> = None;
    let mut file_to_play: Option<PathBuf> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for item in &app.dir_entries {
            let icon = if item.is_dir {
                "📁"
            } else {
                "🎵"
            };
            let resp = ui.selectable_label(false, format!("{icon} {}", item.name));
            if item.is_dir && resp.double_clicked() {
                dir_to_open = Some(item.path.clone());
            } else if !item.is_dir && resp.clicked() {
                file_to_play = Some(item.path.clone());
            }
        }
    });

    if let Some(dir) = dir_to_open {
        scanner::navigate_to(app, dir);
    }
    if let Some(file) = file_to_play {
        app.player.stop();
        let m = parse_track_meta(&file);

        let title = if m.title.is_empty() {
            file.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string()
        } else {
            m.title
        };

        let artist = if m.artist.is_empty() {
            "Unknown Artist".to_string()
        } else {
            m.artist
        };
        let album  = if m.album.is_empty() {
            "Unknown Album".to_string() 
        } else {
            m.album
        };

        let album_dir = file.parent()
            .and_then(|p| if p.file_name().is_some_and(|n| n == "songs") {
                p.parent()
            } else {
                Some(p)
            })
            .and_then(|p| p.file_name()).and_then(|s| s.to_str()).unwrap_or("").to_string();

        app.player.playlist = vec![Track {
            path: file.clone(),
            title,
            artist,
            album,
            album_dir: album_dir.into(),
            track_no: m.track_no,
            disc_no:  m.disc_no,
        }];
    }
}
