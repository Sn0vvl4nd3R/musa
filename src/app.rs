use anyhow::Result;
use eframe::{
    App,
    egui,
    Frame
};
use egui::{
    Color32,
    RichText,
};
use image::GenericImageView;
use std::{
    time::{
        Duration,
        Instant
    },
    fs,
    thread,
    sync::mpsc,
    path::PathBuf,
};

use crate::{
    cover::find_cover_image,
    player::Player,
    track::{
        Track,
        scan_tracks,
        guess_artist_album,
        split_artist_title,
        strip_leading_track_numbers,
    },
    ui::widgets::{
        seekbar,
        accent_button,
        draw_icon_play,
        draw_icon_prev,
        draw_icon_next,
        draw_icon_pause,
        icon_button_circle,
    },
    util::{
        home_dir,
        normalize_ws,
        is_audio_file,
        seconds_to_mmss
    },
    theme::{
        lerp_srgb,
        apply_visuals,
        extract_palette,
        title_from_accent,
        accent_from_palette,
        make_gradient_stops,
    },
};

#[inline]
fn soft_sat(x: f32, knee: f32) -> f32 {
    (knee * x).tanh() / knee.tanh()
}

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

fn srgb_channel_to_linear(u: f32) -> f32 {
    if u <= 0.04045 {
        u / 12.92
    } else {
        ((u + 0.055) / 1.055).powf(2.4)
    }
}
fn rel_luminance(c: Color32) -> f32 {
    let r = srgb_channel_to_linear(c.r() as f32 / 255.0);
    let g = srgb_channel_to_linear(c.g() as f32 / 255.0);
    let b = srgb_channel_to_linear(c.b() as f32 / 255.0);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}
fn best_on(c: Color32) -> Color32 {
    if rel_luminance(c) > 0.5 {
        Color32::from_rgb(5,5,7)
    } else {
        Color32::WHITE
    }
}

struct ThemeAnim {
    active: bool,
    start: Instant,
    dur: Duration,

    from_bg: [Color32; 3],
    to_bg: [Color32; 3],

    from_accent: Color32,
    to_accent: Color32,

    from_title: Color32,
    to_title: Color32,

    from_header: Color32,
    to_header: Color32,
}

impl ThemeAnim {
    fn new() -> Self {
        Self {
            active: false,
            start: Instant::now(),
            dur: Duration::from_millis(420),
            from_bg: [Color32::BLACK; 3],
            to_bg: [Color32::BLACK; 3],
            from_accent: Color32::WHITE,
            to_accent: Color32::WHITE,
            from_title: Color32::WHITE,
            to_title: Color32::WHITE,
            from_header: Color32::WHITE,
            to_header: Color32::WHITE,
        }
    }

    #[inline]
    fn ease(t: f32) -> f32 {
        0.5 - 0.5 * (std::f32::consts::PI * t.clamp(0.0, 1.0)).cos()
    }
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
    cover_rx: Option<mpsc::Receiver<anyhow::Result<(usize, usize, Vec<u8>, String, [Color32; 3])>>>,

    bg_colors: [Color32; 3],
    bg_tex: Option<egui::TextureHandle>,
    accent: Color32,
    title_color: Color32,
    header_color: Color32,

    anim: ThemeAnim,

    scan_rx: Option<mpsc::Receiver<anyhow::Result<Vec<Track>>>>,

    vis_draw: Vec<f32>,
    last_frame: std::time::Instant,
    dt_sec: f32,

    agc_env: f32,
    agc_gain: f32,
    agc_target: f32,
    agc_gain_min: f32,
    agc_gain_max: f32,
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

            bg_colors: make_gradient_stops([
                Color32::from_rgb(36, 36, 40),
                Color32::from_rgb(24, 24, 28),
                Color32::from_rgb(12, 12, 14),
            ]),
            bg_tex: None,
            accent: Color32::from_rgb(120, 160, 255),
            title_color: title_from_accent(Color32::from_rgb(120, 160, 255)),
            header_color: title_from_accent(Color32::from_rgb(120, 160, 255)),

            anim: ThemeAnim::new(),

            scan_rx: None,

            vis_draw: vec![0.0; 120],
            last_frame: std::time::Instant::now(),
            dt_sec: 1.0/60.0,

            agc_env: 0.05,
            agc_gain: 1.0,
            agc_target: 0.55,
            agc_gain_min: 0.8,
            agc_gain_max: 3.0,
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

    fn rebuild_bg_texture(&mut self, ctx: &egui::Context) {
        let n = 1024usize;
        let mut img = egui::ColorImage::new([1, n], self.bg_colors[1]);
        for i in 0..n {
            let t = i as f32 / (n - 1) as f32;
            let c = if t <= 0.5 {
                let tt = t * 2.0;
                lerp_srgb(self.bg_colors[0], self.bg_colors[1], tt)
            } else {
                let tt = (t - 0.5) * 2.0;
                lerp_srgb(self.bg_colors[1], self.bg_colors[2], tt)
            };
            img.pixels[i] = c;
        }
        if let Some(tex) = &mut self.bg_tex {
            tex.set(img, egui::TextureOptions::LINEAR);
        } else {
            self.bg_tex = Some(ctx.load_texture("bg_gradient", img, egui::TextureOptions::LINEAR));
        }
    }

    fn begin_theme_anim(&mut self, to_bg: [Color32;3], to_accent: Color32) {
        self.anim.from_bg = self.bg_colors;
        self.anim.to_bg = to_bg;

        self.anim.from_accent = self.accent;
        self.anim.to_accent = to_accent;

        self.anim.from_title = self.title_color;
        self.anim.to_title = title_from_accent(to_accent);

        self.anim.from_header = self.header_color;
        self.anim.to_header = title_from_accent(to_accent);

        self.anim.start = Instant::now();
        self.anim.dur = Duration::from_millis(420);
        self.anim.active = true;
    }

    fn tick_theme_anim(&mut self, ctx: &egui::Context) {
        if !self.anim.active {
            return;
        }
        let t = (Instant::now() - self.anim.start).as_secs_f32() / (self.anim.dur.as_secs_f32());
        let k = ThemeAnim::ease(t);
        for i in 0..3 {
            self.bg_colors[i] = lerp_srgb(self.anim.from_bg[i], self.anim.to_bg[i], k);
        }
        self.accent = lerp_srgb(self.anim.from_accent, self.anim.to_accent, k);
        self.title_color = lerp_srgb(self.anim.from_title,  self.anim.to_title,  k);
        self.header_color = lerp_srgb(self.anim.from_header, self.anim.to_header, k);

        self.rebuild_bg_texture(ctx);

        if t >= 1.0 {
            self.anim.active = false;
            self.bg_colors = self.anim.to_bg;
            self.accent = self.anim.to_accent;
            self.title_color = self.anim.to_title;
            self.header_color = self.anim.to_header;
            self.rebuild_bg_texture(ctx);
        }
    }

    fn poll_cover_result(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.cover_rx {
            match rx.try_recv() {
                Ok(Ok((w, h, rgba, key, pal))) => {
                    let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                    let tex = ctx.load_texture("cover_art", img, egui::TextureOptions::LINEAR);
                    self.cover_tex = Some(tex);
                    self.cover_id_path = Some(key);

                    let target_bg = make_gradient_stops(pal);
                    let target_accent = accent_from_palette(pal);
                    self.begin_theme_anim(target_bg, target_accent);

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
    }

    fn ui_page_browser(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if accent_button(ui,"Home", self.accent).clicked() {
                self.go_home();
            }
            if accent_button(ui, "Up", self.accent).clicked() {
                self.go_up();
            }
            let mut goto = false;
            ui.label(RichText::new("Path:").strong().color(self.accent));
            let edit = ui.text_edit_singleline(&mut self.current_dir_text);
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                goto = true;
            }
            if accent_button(ui, "Go", self.accent).clicked() {
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
            if accent_button(ui, "Scan this folder", self.accent).clicked() {
                self.start_scan_current();
            }
        });

        if !self.status.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(&self.status).color(Color32::LIGHT_RED));
        }

        ui.add_space(8.0);

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

        self.draw_visualizer_bg(ui, self.dt_sec);

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

            ui.label(RichText::new(title).strong().size(24.0).color(self.title_color));
            if !artist.is_empty() {
                ui.label(RichText::new(artist).size(18.0));
            }
            if !album.is_empty()  {
                ui.label(RichText::new(album).size(16.0).color(Color32::from_gray(210)));
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
                        ui.label(RichText::new(&t.album).size(16.0).color(self.title_color));
                    }
                }
            });

            cols[1].label(RichText::new("Playlist").size(21.0).strong().color(self.header_color));
            let mut select_after: Option<usize> = None;
            let n = self.player.playlist.len();
            let on_accent = best_on(self.accent);

            egui::ScrollArea::vertical().show(&mut cols[1], |ui| {
                for i in 0..n {
                    let t = &self.player.playlist[i];
                    let is_sel = i == self.player.index;

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
                ui.add_sized([time_w, row1_h], egui::Label::new(
                    RichText::new(seconds_to_mmss(pos)).monospace().color(Color32::from_rgb(240,240,246))
                ));
                ui.add_space(gap);
                let seek_w = (ui.available_width() - (time_w + gap)).max(80.0);
                if let Some(new_secs) = seekbar(ui, pos, total, seek_w, 14.0, self.accent) {
                    if let Err(e) = self.player.seek_to_secs(new_secs) {
                        self.status = format!("Seek error: {e}");
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
                let slider_resp = ui.add_sized(
                    [vol_w, 18.0],
                    egui::Slider::new(&mut vol, 0.0..=2.0)
                    .show_value(false)
                );
                let slider_rect = slider_resp.rect;
                ui.painter().rect_stroke(slider_rect, 3.0, egui::Stroke::new(1.0, Color32::WHITE));
                if vol != self.player.volume {
                    self.player.set_volume(vol);
                }
            },
        );

        if !self.status.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new(&self.status).color(Color32::LIGHT_RED));
        }
    }

    fn draw_visualizer_bg(&mut self, ui: &mut egui::Ui, dt: f32) {
        let full = ui.max_rect();

        let band_h = (full.height() * 0.28).clamp(80.0, 200.0);
        let band = egui::Rect::from_min_max(
            egui::pos2(full.left() + 12.0, full.bottom() - band_h - 12.0),
            egui::pos2(full.right() - 12.0, full.bottom() - 12.0),
        );
        if band.height() < 40.0 {
            return;
        }

        let horizon = 6_144;
        let raw = self.player.vis_buffer().take_recent(horizon);
        if raw.len() < 512 {
            return;
        }

        let n = self.vis_draw.len().max(120);
        if self.vis_draw.len() != n {
            self.vis_draw.resize(n, 0.0);
        }

        let fast_w = ((raw.len() as f32 / (n as f32 * 1.5)).clamp(32.0, 256.0)) as usize;
        let slow_w = (fast_w * 4).clamp(fast_w + 8, 2048);
        let hop = ((fast_w as f32) * 0.50).max(12.0) as usize;

        let mut fast: Vec<f32> = Vec::with_capacity(n);
        let mut slow: Vec<f32> = Vec::with_capacity(n);
        let mut idx = raw.len().saturating_sub(fast_w);
        while fast.len() < n && idx >= fast_w {
            let seg_f = &raw[idx - fast_w .. idx];
            let mut sum = 0.0f32;
            let mut peak = 0.0f32;
            for &x in seg_f {
                sum += x*x;
                peak = peak.max(x.abs());
            }
            let rms_f = (sum / fast_w as f32).sqrt();
            fast.push(rms_f * 0.65 + peak * 0.35);

            let start_s = idx.saturating_sub(slow_w);
            let seg_s = &raw[start_s..idx];
            let mut sum2 = 0.0f32;
            for &x in seg_s {
                sum2 += x*x;
            }
            slow.push((sum2 / seg_s.len() as f32).sqrt());

            idx = idx.saturating_sub(hop);
        }
        if fast.len() < 8 {
            return;
        }
        fast.reverse();
        slow.reverse();

        let mut vals: Vec<f32> = Vec::with_capacity(fast.len());
        for i in 0..fast.len() {
            let onset = (fast[i] - slow[i]).max(0.0);
            vals.push(fast[i] * 0.75 + onset * 0.60);
        }

        let mut tmp = vals.clone();
        tmp.sort_by(|a,b| a.partial_cmp(b).unwrap());
        let p90 = tmp[(tmp.len() as f32 * 0.90).floor() as usize].max(1e-4);

        let alpha_env = 1.0 - (-dt / 0.80).exp();
        self.agc_env = self.agc_env + (p90 - self.agc_env) * alpha_env;

        let g_tgt = (self.agc_target / self.agc_env).clamp(self.agc_gain_min, self.agc_gain_max);

        let alpha_gain = 1.0 - (-dt / 0.25).exp();
        self.agc_gain = self.agc_gain + (g_tgt - self.agc_gain) * alpha_gain;

        for v in &mut vals { *v *= self.agc_gain; }

        for v in &mut vals {
            let x = *v;
            *v = soft_sat(x, 1.8);
        }

        let m = vals.len();
        let mut tgt = vec![0.0f32; n];
        for k in 0..n {
            let t  = k as f32 * (m - 1) as f32 / (n - 1) as f32;
            let i0 = t.floor() as usize;
            let i1 = (i0 + 1).min(m - 1);
            let a = vals[i0];
            let b = vals[i1];
            tgt[k] = a + (b - a) * (t - i0 as f32);
        }

        for k in 1..n-1 { tgt[k] = 0.2 * tgt[k-1] + 0.6 * tgt[k] + 0.2 * tgt[k+1]; }

        let tau_up = 0.040;
        let tau_dn = 0.120;
        let a_up = 1.0 - (-dt / tau_up).exp();
        let a_dn = 1.0 - (-dt / tau_dn).exp();
        for i in 0..n {
            let cur = self.vis_draw[i];
            let des = tgt[i];
            let a = if des > cur {
                a_up
            } else {
                a_dn
            };
            self.vis_draw[i] = cur + (des - cur) * a;
        }
        for k in 1..n-1 {
            self.vis_draw[k] = (self.vis_draw[k-1] + 2.0*self.vis_draw[k] + self.vis_draw[k+1]) * 0.25;
        }

        let painter = ui.painter_at(band);
        let h = band.height() * 1.02;
        let baseline = band.bottom() - 4.0;
        let step = band.width() / (n as f32 - 1.0);

        let mut line: Vec<egui::Pos2> = Vec::with_capacity(n);
        for i in 0..n {
            let x = band.left() + i as f32 * step;
            let y = (baseline - h * self.vis_draw[i]).clamp(band.top(), baseline);
            line.push(egui::pos2(x, y));
        }

        let glow = egui::Stroke::new(6.0, egui::Color32::from_rgba_unmultiplied(
            self.accent.r(), self.accent.g(), self.accent.b(), 42));
        painter.add(egui::Shape::line(line.clone(), glow));

        let contour = egui::Stroke::new(1.8, egui::Color32::from_rgba_unmultiplied(255,255,255,170));
        painter.add(egui::Shape::line(line, contour));
    }

}

fn paint_bg_gradient(ctx: &egui::Context, tex: &Option<egui::TextureHandle>, fallback: [Color32; 3]) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    if let Some(t) = tex {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(t.id(), rect, uv, Color32::WHITE);
    } else {
        painter.rect_filled(rect, 0.0, fallback[1]);
    }
}

impl App for MusaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.tick_theme_anim(ctx);

        let now = std::time::Instant::now();
        self.dt_sec = (now - self.last_frame).as_secs_f32().clamp(0.001, 0.05);
        self.last_frame = now;

        apply_visuals(ctx, self.accent);

        if self.bg_tex.is_none() {
            self.rebuild_bg_texture(ctx);
        }
        paint_bg_gradient(ctx, &self.bg_tex, self.bg_colors);

        self.poll_cover_result(ctx);
        self.poll_scan_result(ctx);

        egui::TopBottomPanel::top("musa_top")
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.nav_tabs(ui);
            });

        egui::TopBottomPanel::bottom("musa_bottom")
            .frame(egui::Frame::none())
            .resizable(false)
            .exact_height(84.0)
            .show(ctx, |ui| self.bottom_controls(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| match self.view {
                UiView::Browser => self.ui_page_browser(ui),
                UiView::Player => self.ui_page_player(ui),
                UiView::Playlist => self.ui_page_playlist(ui),
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
