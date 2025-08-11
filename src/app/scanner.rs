use crate::track::scan_tracks;
use crate::app::types::DirEntryItem;

use super::{
    UiView,
    MusaApp,
};

use crate::util::{
    home_dir,
    is_audio_file,
};

use std::{
    fs,
    thread,
    sync::mpsc,
    path::{
        Path,
        PathBuf
    },
};

pub(super) fn read_dir_items(dir: &Path) -> Vec<DirEntryItem> {
    let mut out = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for e in read.flatten() {
            let path = e.path();
            let is_dir = path.is_dir();
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            if is_dir || is_audio_file(&path) {
                out.push(DirEntryItem {
                    name,
                    path,
                    is_dir
                });
            }
        }
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    out
}

pub(super) fn navigate_to(app: &mut MusaApp, path: PathBuf) {
    if path.is_dir() {
        app.current_dir = path;
        app.current_dir_text = app.current_dir.display().to_string();
        app.dir_entries = read_dir_items(&app.current_dir);
    } else {
        app.status = "Not a directory".into();
    }
}

pub(super) fn go_home(app: &mut MusaApp) {
    navigate_to(app, home_dir());
}

pub(super) fn go_up(app: &mut MusaApp) {
    if let Some(p) = app.current_dir.parent() {
        navigate_to(app, p.to_path_buf());
    }
}

pub(super) fn start_scan_current(app: &mut MusaApp) {
    if app.scan_rx.is_some() {
        app.status = "Scanning already in progress...".into();
        return;
    }
    let dir = app.current_dir.clone();
    let (tx, rx) = mpsc::channel();
    app.scan_rx = Some(rx);
    app.status.clear();
    thread::spawn(move || {
        let _ = tx.send(scan_tracks(&dir));
    });
}

pub(super) fn poll_scan_result(app: &mut MusaApp, ctx: &egui::Context) {
    if let Some(rx) = &app.scan_rx {
        match rx.try_recv() {
            Ok(Ok(list)) => {
                app.player.stop();
                app.player.playlist = list;
                app.player.index = 0;
                if let Err(e) = app.player.play_current() {
                    app.status = format!("Playback error: {e}");
                } else {
                    crate::app::cover::update_cover_from_current_track(app);
                    app.view = UiView::Playlist;
                    app.status.clear();
                }
                app.scan_rx = None;
                ctx.request_repaint();
            }
            Ok(Err(e)) => {
                app.status = format!("Scan error: {e}");
                app.scan_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                app.status = "Scan interrupted".into();
                app.scan_rx = None;
            }
        }
    }
}
