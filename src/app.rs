use anyhow::Result;
use eframe::{egui,
    App,
    Frame
};
use image::GenericImageView;
use egui::{
    Color32,
    RichText
};
use std::{
    fs,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::Duration,
};

use crate::{
    cover::find_cover_image,
    player::Player,
    track::{
        scan_tracks,
        guess_artist_album,
        Track,
        split_artist_title,
        strip_leading_track_numbers
    },
    util::{
        home_dir,
        seconds_to_mmss,
        normalize_ws
    },
    ui::widgets::{
        icon_button_circle,
        draw_icon_prev,
        draw_icon_next,
        draw_icon_pause,
        draw_icon_play,
        seekbar
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiView {
    Browser,
    Player,
    Playlist
}

#[derive(Clone)]
struct DirEntryItem {
    name: String,
    path: PathBuf,
    is_dir: bool
}

fn read_dir_items(dir: &std::path::Path) -> Vec<DirEntryItem> {
    let mut out = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for e in read.flatten() {
            let path = e.path();
            let is_dir = path.is_dir();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
            if is_dir || crate::util::is_audio_file(&path) {
                out.push(DirEntryItem { name, path, is_dir });
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

pub struct MusaApp {
    pub player: Player,
    view: UiView,
    status: String,
    current_dir: PathBuf,
    current_dir_text: String,
    dir_entries: Vec<DirEntryItem>,
    cover_path: Option<PathBuf>,
    cover_id_path: Option<String>,
    cover_tex: Option<egui::TextureHandle>,
    cover_rx: Option<mpsc::Receiver<anyhow::Result<(usize, usize, Vec<u8>, String)>>>,
    scan_rx: Option<mpsc::Receiver<anyhow::Result<Vec<Track>>>>,
}

impl MusaApp {
    pub fn new() -> Result<Self> {
        let start_dir = home_dir();
        Ok(Self {
            player: Player::new(),
            view: UiView::Player,
            status: String::new(),
            current_dir: start_dir.clone(),
            current_dir_text: start_dir.display().to_string(),
            dir_entries: read_dir_items(&start_dir),
            cover_path: None,
            cover_id_path: None,
            cover_tex: None,
            cover_rx: None,
            scan_rx: None,
        })
    }

    fn navigate_to(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.current_dir = path;
            self.current_dir_text = self.current_dir.display().to_string();
            self.dir_entries = read_dir_items(&self.current_dir);
        } else {
            self.status = "Not a directory".into();
        }
    }
    fn go_home(&mut self) {
        self.navigate_to(home_dir());
    }
    fn go_up(&mut self) {
        if let Some(p) = self.current_dir.parent() {
            self.navigate_to(p.to_path_buf());
        }
    }

    fn start_scan_current(&mut self) {
        if self.scan_rx.is_some() {
            self.status = "Scanning already in progress...".into();
            return;
        }
        let dir = self.current_dir.clone();
        let (tx, rx) = mpsc::channel();
        self.scan_rx = Some(rx);
        self.status.clear();
        thread::spawn(move || {
            let _ = tx.send(scan_tracks(&dir));
        });
    }

    fn poll_scan_result(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.scan_rx {
            match rx.try_recv() {
                Ok(Ok(list)) => {
                    self.player.stop();
                    self.player.playlist = list;
                    self.player.index = 0;
                    if let Err(e) = self.player.play_current() {
                        self.status = format!("Playback error: {e}");
                    } else {
                        self.update_cover_from_current_track();
                        self.view = UiView::Playlist;
                        self.status.clear();
                    }
                    self.scan_rx = None;
                    ctx.request_repaint();
                }
                Ok(Err(e)) => {
                    self.status = format!("Scan error: {e}");
                    self.scan_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = "Scan interrupted".into();
                    self.scan_rx = None;
                }
            }
        }
    }

    fn update_cover_from_current_track(&mut self) {
        if let Some(dir) = self.player.current_album_dir() {
            self.cover_path = find_cover_image(&dir);
            self.cover_id_path = None;
            self.request_cover_load();
        } else {
            self.cover_path = None;
            self.cover_tex = None;
            self.cover_id_path = None;
            self.cover_rx = None;
        }
    }

    fn request_cover_load(&mut self) {
        if let Some(path) = &self.cover_path {
            let key = path.display().to_string();
            if self.cover_id_path.as_deref() == Some(&key) || self.cover_rx.is_some() {
                return;
            }
            let (tx, rx) = mpsc::channel();
            self.cover_rx = Some(rx);
            let p = path.clone();
            let k = key.clone();
            thread::spawn(move || {
                let res: anyhow::Result<(usize, usize, Vec<u8>, String)> = (|| {
                    let img = image::open(&p)?;
                    let (w, h) = img.dimensions();
                    Ok((w as usize, h as usize, img.to_rgba8().into_raw(), k))
                })();
                let _ = tx.send(res);
            });
        }
    }

    fn poll_cover_result(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.cover_rx {
            match rx.try_recv() {
                Ok(Ok((w, h, rgba, key))) => {
                    let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                    let tex = ctx.load_texture("cover_art", img, egui::TextureOptions::LINEAR);
                    self.cover_tex = Some(tex);
                    self.cover_id_path = Some(key);
                    self.cover_rx = None;
                }
                Ok(Err(e)) => {
                    self.status = format!("Cover load error: {e}");
                    self.cover_tex = None;
                    self.cover_id_path = None;
                    self.cover_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.cover_tex = None;
                    self.cover_id_path = None;
                    self.cover_rx = None;
                }
            }
        }
    }

    fn nav_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut tab = |label: &str, v: UiView| {
                let selected = self.view == v;
                if ui.selectable_label(selected, label).clicked() {
                    self.view = v;
                }
            };
            tab("Browser", UiView::Browser);
            tab("Player", UiView::Player);
            tab("Playlist", UiView::Playlist);
        });
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);
    }

    fn ui_page_browser(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Home").clicked() {
                self.go_home();
            }
            if ui.button("Up").clicked() {
                self.go_up();
            }
            let mut goto = false;
            ui.label("Path:");
            let edit = ui.text_edit_singleline(&mut self.current_dir_text);
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                goto = true;
            }
            if ui.button("Go").clicked() {
                goto = true;
            }
            if goto {
                let p = PathBuf::from(self.current_dir_text.clone());
                if p.is_dir() {
                    self.navigate_to(p);
                    self.status.clear();
                } else {
                    self.status = "Path does not exist or is not a directory".into();
                }
            }
            if ui.button("Scan this folder").clicked() {
                self.start_scan_current();
            }
        });

        if !self.status.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(&self.status).color(Color32::LIGHT_RED));
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        let mut dir_to_open: Option<PathBuf> = None;
        let mut file_to_play: Option<PathBuf> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in &self.dir_entries {
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
            self.navigate_to(dir);
        }
        if let Some(file) = file_to_play {
            self.player.stop();
            let (artist, album, album_dir, disc_no) = guess_artist_album(&file);
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let stem = normalize_ws(stem);
            let (leading_no, s1) = strip_leading_track_numbers(&stem);
            let (artist_from_file, s2) = split_artist_title(&s1);
            let title = normalize_ws(s2);
            let artist_final = artist_from_file.unwrap_or(artist);
            self.player.playlist = vec![Track {
                path: file.clone(),
                title,
                artist: artist_final,
                album,
                album_dir,
                track_no: leading_no,
                disc_no,
            }];
            self.player.index = 0;
            if let Err(e) = self.player.play_current() {
                self.status = format!("Playback error: {e}");
            } else {
                self.update_cover_from_current_track();
                self.view = UiView::Player;
            }
        }
    }

    fn ui_page_player(&mut self, ui: &mut egui::Ui) {
        if let Err(e) = self.player.auto_advance_if_needed() {
            self.status = format!("Auto-advance error: {e}");
        }

        ui.vertical_centered(|ui| {
            let avail = ui.available_size();
            let reserve_under_image = 160.0;
            let max_side = 460.0;
            let side_from_height = (avail.y - reserve_under_image).max(100.0);
            let side = side_from_height.min(avail.x).min(max_side);

            if let Some(tex) = &self.cover_tex {
                let size = tex.size();
                let ratio = (side / size[0] as f32)
                    .min(side / size[1] as f32)
                    .min(1.0);
                ui.image((tex.id(), egui::vec2(size[0] as f32 * ratio, size[1] as f32 * ratio)));
            } else {
                ui.label(RichText::new("No cover").color(Color32::GRAY));
            }

            ui.add_space(8.0);
            let (title, artist, album) = if let Some(t) = self.player.current_track() {
                t.display_now_playing()
            } else {
                ("—".into(), "".into(), "".into())
            };

            ui.label(RichText::new(title).strong().size(24.0).color(Color32::LIGHT_GREEN));
            if !artist.is_empty() {
                ui.label(RichText::new(artist).size(18.0).color(Color32::WHITE));
            }
            if !album.is_empty() {
                ui.label(RichText::new(album).size(16.0).color(Color32::GRAY));
            }
        });

        if !self.status.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(&self.status).color(Color32::LIGHT_RED));
        }
    }

    fn ui_page_playlist(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            cols[0].with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                let avail = ui.available_size();
                let reserve = 60.0;
                let side_from_height = (avail.y - reserve).max(100.0);
                let side = side_from_height.min(avail.x).min(360.0);
                if let Some(tex) = &self.cover_tex {
                    let size = tex.size();
                    let ratio = (side / size[0] as f32).min(side / size[1] as f32).min(1.0);
                    ui.image((tex.id(), egui::vec2(size[0] as f32 * ratio, size[1] as f32 * ratio)));
                } else {
                    ui.label(RichText::new("No cover").color(Color32::GRAY));
                }
                ui.add_space(6.0);
                if let Some(t) = self.player.current_track() {
                    if !t.album.is_empty() {
                        ui.label(RichText::new(&t.album).size(16.0).color(Color32::LIGHT_GREEN));
                    }
                }
            });

            cols[1].heading("Playlist");
            let mut select_after: Option<usize> = None;
            let n = self.player.playlist.len();
            egui::ScrollArea::vertical().show(&mut cols[1], |ui| {
                for i in 0..n {
                    let t = &self.player.playlist[i];
                    let mut txt = RichText::new(format!("{:>3}. {}", i + 1, t.display_line()));
                    if i == self.player.index {
                        txt = txt.color(Color32::YELLOW).strong();
                    }
                    if ui.selectable_label(i == self.player.index, txt).clicked() {
                        select_after = Some(i);
                    }
                }
            });
            if let Some(i) = select_after {
                self.player.index = i;
                if let Err(e) = self.player.play_current() {
                    self.status = format!("Playback error: {e}");
                } else {
                    self.update_cover_from_current_track();
                }
            }
        });
    }

    fn bottom_controls(&mut self, ui: &mut egui::Ui) {
        let total = self.player.current_total_secs();
        let have_total = total.is_finite() && total > 0.0;
        let mut pos = self.player.current_pos().as_secs_f32();
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
                ui.add_sized([time_w, row1_h], egui::Label::new(RichText::new(seconds_to_mmss(pos)).monospace()));
                ui.add_space(gap);
                let seek_w = (ui.available_width() - (time_w + gap)).max(80.0);
                if let Some(new_secs) = seekbar(ui, pos, total, seek_w, 14.0) {
                    if let Err(e) = self.player.seek_to_secs(new_secs) {
                        self.status = format!("Seek error: {e}");
                    }
                }
                ui.add_space(gap);
                ui.add_sized([time_w, row1_h], egui::Label::new(
                    RichText::new(seconds_to_mmss(if have_total { total } else { f32::NAN })).monospace(),
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
                    if let Err(e) = self.player.prev() {
                        self.status = format!("Prev error: {e}");
                    } else {
                        self.update_cover_from_current_track();
                    }
                }

                ui.add_space(gap);

                let label_tt = if self.player.is_playing() {
                    "Pause"
                } else {
                    "Play"
                };
                let play_resp = icon_button_circle(ui, play_d, label_tt, |p, r, c| {
                    if self.player.is_playing() {
                        draw_icon_pause(p, r, c)
                    } else {
                        draw_icon_play(p, r, c)
                    }
                });
                if play_resp.clicked() {
                    self.player.toggle_pause();
                }

                ui.add_space(gap);

                let next_resp = icon_button_circle(ui, next_d, "Next track", |p, r, c| draw_icon_next(p, r, c));
                if next_resp.clicked() {
                    if let Err(e) = self.player.next() {
                        self.status = format!("Next error: {e}");
                    } else {
                        self.update_cover_from_current_track();
                    }
                }

                let used = left_pad + center_block_w + gap;
                let remain = (w_all - used - vol_w).max(0.0);
                ui.allocate_space(egui::vec2(remain, 0.0));

                let mut vol = self.player.volume;
                if ui.add_sized([vol_w, 18.0], egui::Slider::new(&mut vol, 0.0..=2.0).text("Volume").show_value(false)).changed() {
                    self.player.set_volume(vol);
                }
            },
        );

        if !self.status.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new(&self.status).color(Color32::LIGHT_RED));
        }
    }
}

impl App for MusaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.poll_cover_result(ctx);
        self.poll_scan_result(ctx);

        egui::TopBottomPanel::top("musa_top")
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.nav_tabs(ui);
            });

        egui::TopBottomPanel::bottom("musa_bottom")
            .resizable(false)
            .exact_height(84.0)
            .show(ctx, |ui| {
                self.bottom_controls(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            UiView::Browser => self.ui_page_browser(ui),
            UiView::Player => self.ui_page_player(ui),
            UiView::Playlist=> self.ui_page_playlist(ui),
        });

        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let next_in = if focused {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(250)
        };
        ctx.request_repaint_after(next_in);
    }
}
