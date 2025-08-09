use anyhow::{anyhow, Context, Result};
use eframe::{egui, App, Frame, Renderer};
use egui::{Color32, RichText, Vec2};
use image::GenericImageView;
use regex::Regex;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use walkdir::WalkDir;

fn is_audio_file(p: &Path) -> bool {
    match p.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
        Some(ext) => matches!(ext.as_str(), "mp3" | "flac" | "ogg" | "opus" | "wav" | "aiff" | "aif" | "alac" | "m4a"),
        None => false,
    }
}
fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/"))
}
fn seconds_to_mmss(secs: f32) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".to_string();
    }
    let s = secs as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn strip_year_prefix(mut s: String) -> String {
    let re1 = Regex::new(r"(?i)^\s*\d{4}\s*[-_.]\s*").unwrap();
    let re2 = Regex::new(r"(?i)^\s*[\(\[]\s*\d{4}\s*[\)\]]\s*[-_.]?\s*").unwrap();
    s = re1.replace(&s, "").to_string();
    s = re2.replace(&s, "").to_string();
    s.trim().to_string()
}
fn strip_year_suffix(mut s: String) -> String {
    let re = Regex::new(r"(?i)\s*[\(\[]\s*\d{4}\s*[\)\]]\s*$").unwrap();
    s = re.replace(&s, "").to_string();
    s.trim().to_string()
}
fn strip_tech_brackets(mut s: String) -> String {
    let re = Regex::new(
        r#"(?ix)\s*[\(\[][^)\]]*(?:kHz|Hz|bit|kbps|VBR|CBR|FLAC|ALAC|MP3|AAC|OGG|OPUS|DSD|mono|stereo)[^)\]]*[\)\]]"#,
    )
    .unwrap();
    loop {
        let t = re.replace(&s, "").to_string();
        if t == s {
            break;
        }
        s = t;
    }
    s.trim().to_string()
}
fn strip_artist_prefix(mut album: String, artist: &str) -> String {
    for sep in [" - ", " – ", " — "] {
        let pref = format!("{}{}", artist, sep);
        if album.to_ascii_lowercase().starts_with(&pref.to_ascii_lowercase()) {
            album = album[pref.len()..].to_string();
            break;
        }
    }
    album
}
fn normalize_ws(mut s: String) -> String {
    s = s.replace('_', " ");
    let re_ws = Regex::new(r"\s+").unwrap();
    s = re_ws.replace_all(&s, " ").to_string();
    s.trim().to_string()
}
fn clean_album_for_display(raw_album: &str, artist: &str) -> String {
    let mut s = raw_album.trim().to_string();
    s = strip_year_prefix(s);
    s = strip_year_suffix(s);
    s = strip_tech_brackets(s);
    s = strip_artist_prefix(s, artist);
    normalize_ws(s)
}

fn strip_leading_track_numbers(stem: &str) -> (Option<u32>, String) {
    let mut s = stem.to_string();
    let mut first: Option<u32> = None;
    let re_delim = Regex::new(r#"^\s*(\d{1,3})\s*([)\]\-._]+)\s*"#).unwrap();
    let re_zero_space = Regex::new(r#"^\s*(0\d{1,2})\s+"#).unwrap();
    let re_cleanup = Regex::new(r#"^\s*[)\]\-._]+\s*"#).unwrap();
    loop {
        if let Some(cap) = re_delim.captures(&s) {
            if first.is_none() {
                if let Ok(n) = cap[1].parse::<u32>() {
                    first = Some(n);
                }
            }
            let end = cap.get(0).unwrap().end();
            s = s[end..].to_string();
            loop {
                if let Some(m) = re_cleanup.find(&s) {
                    if m.start() == 0 {
                        s = s[m.end()..].to_string();
                        continue;
                    }
                }
                break;
            }
            continue;
        }
        if let Some(cap) = re_zero_space.captures(&s) {
            if first.is_none() {
                if let Ok(n) = cap[1].parse::<u32>() {
                    first = Some(n);
                }
            }
            let end = cap.get(0).unwrap().end();
            s = s[end..].to_string();
            continue;
        }
        break;
    }
    (first, s.trim().to_string())
}

fn split_artist_title(s: &str) -> (Option<String>, String) {
    for sep in [" - ", " – ", " — "] {
        if let Some(idx) = s.find(sep) {
            let artist = s[..idx].trim();
            let title = s[idx + sep.len()..].trim();
            if !artist.is_empty() && !title.is_empty() {
                return (Some(artist.to_string()), title.to_string());
            }
        }
    }
    (None, s.trim().to_string())
}
fn parse_disc_folder(name: &str) -> Option<u32> {
    let re = Regex::new(r"(?i)^(?:cd|disc)\s*[-_ ]?\s*(\d{1,3})$").unwrap();
    re.captures(name.trim())
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}
fn album_dir_and_disc(file: &Path) -> (PathBuf, Option<u32>) {
    let mut dir = file.parent().unwrap_or_else(|| Path::new("/"));
    let mut disc = None;
    if let Some(name) = dir.file_name().and_then(|s| s.to_str()) {
        if let Some(d) = parse_disc_folder(name) {
            disc = Some(d);
            dir = dir.parent().unwrap_or(dir);
        }
    }
    (dir.to_path_buf(), disc)
}
fn guess_artist_album(file: &Path) -> (String, String, PathBuf, Option<u32>) {
    let (album_dir, disc_no) = album_dir_and_disc(file);
    let album_name_raw = album_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let artist = album_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let album = clean_album_for_display(album_name_raw, &artist);
    (artist, album, album_dir, disc_no)
}

#[derive(Clone)]
struct Track {
    path: PathBuf,
    title: String,
    artist: String,
    album: String,
    album_dir: PathBuf,
    track_no: Option<u32>,
    disc_no: Option<u32>,
}
impl Track {
    fn display_line(&self) -> String {
        self.title.clone()
    }
    fn display_now_playing(&self) -> (String, String, String) {
        (self.title.clone(), self.artist.clone(), self.album.clone())
    }
}

fn scan_tracks(root: &Path) -> Result<Vec<Track>> {
    if !root.exists() {
        return Err(anyhow!("Path does not exist: {}", root.display()));
    }
    let mut files: Vec<PathBuf> = Vec::new();
    if root.is_file() {
        if is_audio_file(root) {
            files.push(root.canonicalize()?);
        }
    } else {
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() && is_audio_file(p) {
                files.push(p.canonicalize()?);
            }
        }
    }
    if files.is_empty() {
        return Err(anyhow!("No audio files found in {}", root.display()));
    }
    files.sort();

    let mut out = Vec::with_capacity(files.len());
    for path in files {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let stem = normalize_ws(stem);
        let (leading_no, s1) = strip_leading_track_numbers(&stem);
        let (artist_from_file, s2) = split_artist_title(&s1);
        let title = normalize_ws(s2);
        let (artist_from_dir, album, album_dir, disc_no) = guess_artist_album(&path);
        let artist = artist_from_file.unwrap_or_else(|| artist_from_dir.clone());
        out.push(Track { path, title, artist, album, album_dir, track_no: leading_no, disc_no });
    }

    out.sort_by(|a, b| {
        let ad = a.disc_no.unwrap_or(0);
        let bd = b.disc_no.unwrap_or(0);
        match ad.cmp(&bd) {
            std::cmp::Ordering::Equal => {
                let at = a.track_no.unwrap_or(10_000);
                let bt = b.track_no.unwrap_or(10_000);
                match at.cmp(&bt) {
                    std::cmp::Ordering::Equal => a.path.cmp(&b.path),
                    x => x,
                }
            }
            x => x,
        }
    });

    Ok(out)
}

fn find_cover_image(dir: &Path) -> Option<PathBuf> {
    const NAMES: &[&str] = &[
        "cover.jpg", "cover.png", "cover.jpeg", "cover.webp", "folder.jpg", "folder.png", "folder.jpeg", "folder.webp",
        "front.jpg", "front.png", "front.jpeg", "front.webp",
    ];
    for n in NAMES {
        let p = dir.join(n);
        if p.exists() && p.is_file() {
            return Some(p);
        }
    }
    if let Ok(read) = fs::read_dir(dir) {
        for e in read.flatten() {
            let p = e.path();
            if let Some(ext) = p.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
                if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif" | "tiff") {
                    return Some(p);
                }
            }
        }
    }
    None
}

struct Player {
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    playlist: Vec<Track>,
    index: usize,
    volume: f32,
    track_total: Option<Duration>,
    pos_base: Duration,
    pos_started_at: Option<Instant>,
}
impl Player {
    fn new(handle: OutputStreamHandle) -> Self {
        Self {
            handle,
            sink: None,
            playlist: Vec::new(),
            index: 0,
            volume: 1.0,
            track_total: None,
            pos_base: Duration::ZERO,
            pos_started_at: None,
        }
    }
    fn current_track(&self) -> Option<&Track> {
        self.playlist.get(self.index)
    }
    fn current_path(&self) -> Option<&Path> {
        self.current_track().map(|t| t.path.as_path())
    }
    fn current_album_dir(&self) -> Option<PathBuf> {
        self.current_track().map(|t| t.album_dir.clone())
    }
    fn is_playing(&self) -> bool {
        self.sink.as_ref().map(|s| !s.is_paused()).unwrap_or(false)
    }
    fn current_total_secs(&self) -> f32 {
        self.track_total.map(|d| d.as_secs_f32()).unwrap_or(f32::NAN)
    }
    fn current_pos(&self) -> Duration {
        if let Some(st) = self.pos_started_at {
            if self.is_playing() {
                return self.pos_base.saturating_add(st.elapsed());
            }
        }
        self.pos_base
    }
    fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 2.0);
        if let Some(s) = &self.sink {
            s.set_volume(self.volume);
        }
    }
    fn stop(&mut self) {
        if let Some(s) = &self.sink {
            s.stop();
        }
        self.sink = None;
        self.pos_started_at = None;
    }
    fn start_sink_with_source(&mut self, dec: Decoder<BufReader<File>>, skip: Duration) -> Result<()> {
        self.track_total = dec.total_duration();
        let src = dec.skip_duration(skip).convert_samples::<f32>();
        let sink = Sink::try_new(&self.handle).map_err(|e| anyhow!("Cannot create audio sink: {e}"))?;
        sink.set_volume(self.volume);
        sink.append(src);
        sink.play();
        self.sink = Some(sink);
        self.pos_base = skip;
        self.pos_started_at = Some(Instant::now());
        Ok(())
    }
    fn play_current_from(&mut self, pos: Duration) -> Result<()> {
        let Some(p) = self.current_path() else { return Ok(()); };
        let dec = open_decoder(p)?;
        self.stop();
        self.start_sink_with_source(dec, pos)
    }
    fn play_current(&mut self) -> Result<()> {
        self.play_current_from(Duration::ZERO)
    }
    fn toggle_pause(&mut self) {
        if let Some(s) = &self.sink {
            if s.is_paused() {
                s.play();
                self.pos_started_at = Some(Instant::now());
            } else {
                let now_pos = if let Some(st) = self.pos_started_at {
                    self.pos_base.saturating_add(st.elapsed())
                } else {
                    self.pos_base
                };
                self.pos_base = now_pos;
                self.pos_started_at = None;
                s.pause();
            }
        }
    }
    fn seek_to_secs(&mut self, secs: f32) -> Result<()> {
        if !secs.is_finite() || secs < 0.0 {
            return Ok(());
        }
        self.play_current_from(Duration::from_secs_f32(secs))
    }
    fn next(&mut self) -> Result<()> {
        if self.playlist.is_empty() {
            return Ok(());
        }
        self.index = (self.index + 1) % self.playlist.len();
        self.play_current()
    }
    fn prev(&mut self) -> Result<()> {
        if self.playlist.is_empty() {
            return Ok(());
        }
        self.index = if self.index == 0 {
            self.playlist.len() - 1
        } else {
            self.index - 1
        };
        self.play_current()
    }
    fn auto_advance_if_needed(&mut self) -> Result<()> {
        if let Some(s) = &self.sink {
            if s.empty() && !self.playlist.is_empty() {
                self.index = (self.index + 1) % self.playlist.len();
                return self.play_current();
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiView {
    Browser,
    Player,
    Playlist,
}

#[derive(Clone)]
struct DirEntryItem {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

struct MusaApp {
    _stream: OutputStream,
    player: Player,
    view: UiView,
    status: String,
    current_dir: PathBuf,
    current_dir_text: String,
    dir_entries: Vec<DirEntryItem>,
    cover_path: Option<PathBuf>,
    cover_id_path: Option<String>,
    cover_tex: Option<egui::TextureHandle>,
    cover_rx: Option<mpsc::Receiver<Result<(usize, usize, Vec<u8>, String)>>>,
    scan_rx: Option<mpsc::Receiver<Result<Vec<Track>>>>,
}

fn read_dir_items(dir: &Path) -> Vec<DirEntryItem> {
    let mut out = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for e in read.flatten() {
            let path = e.path();
            let is_dir = path.is_dir();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
            if is_dir || is_audio_file(&path) {
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

fn icon_button_circle(
    ui: &mut egui::Ui,
    diameter: f32,
    tooltip: &str,
    draw_icon: impl FnOnce(&egui::Painter, egui::Rect, Color32),
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::click());
    let painter = ui.painter_at(rect);

    let visuals = ui.style().visuals.clone();
    let w_inactive = &visuals.widgets.inactive;
    let w_hovered  = &visuals.widgets.hovered;
    let w_active   = &visuals.widgets.active;

    let (bg_fill, bg_stroke, fg) = if resp.is_pointer_button_down_on() {
        (w_active.bg_fill, w_active.bg_stroke, w_active.fg_stroke.color)
    } else if resp.hovered() {
        (w_hovered.bg_fill, w_hovered.bg_stroke, w_hovered.fg_stroke.color)
    } else {
        (w_inactive.bg_fill, w_inactive.bg_stroke, w_inactive.fg_stroke.color)
    };

    let center = rect.center();
    let radius = diameter * 0.5;
    painter.circle_filled(center, radius, bg_fill);
    painter.circle_stroke(center, radius, bg_stroke);

    let icon_rect = rect.shrink(diameter * 0.28);
    draw_icon(&painter, icon_rect, fg);

    resp.on_hover_text(tooltip)
}

fn draw_icon_play(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let left = r.left();
    let right = r.right();
    let top = r.top();
    let bot = r.bottom();
    let cx = r.center().x;
    let pad_l = (cx - left) * 0.15;
    let p1 = egui::pos2(left + pad_l, top);
    let p2 = egui::pos2(right, r.center().y);
    let p3 = egui::pos2(left + pad_l, bot);
    p.add(egui::Shape::convex_polygon(vec![p1, p2, p3], color, egui::Stroke::new(0.0, color)));
}

fn draw_icon_pause(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let t = r.width() * 0.22;
    let gap = t * 0.45;
    let total = t * 2.0 + gap;
    let x0 = r.center().x - total * 0.5;
    let rect1 = egui::Rect::from_min_max(egui::pos2(x0, r.top()), egui::pos2(x0 + t, r.bottom()));
    let rect2 = egui::Rect::from_min_max(egui::pos2(x0 + t + gap, r.top()), egui::pos2(x0 + t + gap + t, r.bottom()));
    p.rect_filled(rect1, 1.0, color);
    p.rect_filled(rect2, 1.0, color);
}

fn draw_icon_next(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let rr = r.shrink(r.width() * 0.05);
    let tw = rr.width() * 0.30;
    let gap = tw * 0.30;
    let total = tw * 2.0 + gap;
    let x_left = rr.center().x - total * 0.5;
    let y_top = rr.top();
    let y_mid = rr.center().y;
    let y_bot = rr.bottom();

    let tri1 = vec![
        egui::pos2(x_left, y_top),
        egui::pos2(x_left + tw, y_mid),
        egui::pos2(x_left, y_bot),
    ];
    let x2 = x_left + tw + gap;
    let tri2 = vec![
        egui::pos2(x2, y_top),
        egui::pos2(x2 + tw, y_mid),
        egui::pos2(x2, y_bot),
    ];
    p.add(egui::Shape::convex_polygon(tri1, color, egui::Stroke::new(0.0, color)));
    p.add(egui::Shape::convex_polygon(tri2, color, egui::Stroke::new(0.0, color)));
}

fn draw_icon_prev(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let rr = r.shrink(r.width() * 0.05);
    let tw = rr.width() * 0.30;
    let gap = tw * 0.30;
    let total = tw * 2.0 + gap;
    let x_right = rr.center().x + total * 0.5;
    let y_top = rr.top();
    let y_mid = rr.center().y;
    let y_bot = rr.bottom();

    let tri1 = vec![
        egui::pos2(x_right, y_top),
        egui::pos2(x_right - tw, y_mid),
        egui::pos2(x_right, y_bot),
    ];
    let x2 = x_right - tw - gap;
    let tri2 = vec![
        egui::pos2(x2, y_top),
        egui::pos2(x2 - tw, y_mid),
        egui::pos2(x2, y_bot),
    ];
    p.add(egui::Shape::convex_polygon(tri1, color, egui::Stroke::new(0.0, color)));
    p.add(egui::Shape::convex_polygon(tri2, color, egui::Stroke::new(0.0, color)));
}

impl MusaApp {
    fn new() -> Result<Self> {
        let (stream, handle) = OutputStream::try_default().map_err(|e| anyhow!("Audio device not found: {e}"))?;
        let start_dir = home_dir();
        Ok(Self {
            _stream: stream,
            player: Player::new(handle),
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
            self.status = "Scanning is already running…".into();
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
                        self.status = format!("Start error: {e}");
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
                    self.status = "Scan aborted".into();
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
                let res: Result<(usize, usize, Vec<u8>, String)> = (|| {
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

    fn seekbar_sized(&self, ui: &mut egui::Ui, pos: f32, total: f32, width: f32, height: f32) -> Option<f32> {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(width.max(60.0), height), egui::Sense::click_and_drag());
        ui.expand_to_include_rect(rect);

        let painter = ui.painter_at(rect);
        let bg = Color32::from_gray(115);
        let border = Color32::from_gray(165);
        let played = Color32::WHITE;

        painter.rect_filled(rect, 6.0, bg);
        painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, border));

        let have_total = total.is_finite() && total > 0.0;
        let denom = if have_total { total } else { (pos + 1.0).max(1.0) };
        let frac = (pos / denom).clamp(0.0, 1.0);
        let w = rect.left() + rect.width() * frac;
        let played_rect = egui::Rect::from_min_max(rect.min, egui::pos2(w, rect.bottom()));

        painter.rect_filled(played_rect, 6.0, played);

        let hover = resp.hovered() || resp.dragged();
        let r = if hover { 7.5 } else { 6.5 };
        let center = egui::pos2(w, rect.center().y);

        painter.circle_filled(center, r, Color32::WHITE);
        painter.circle_stroke(center, r, egui::Stroke::new(1.0, border));
        if resp.clicked() || resp.dragged() {
            if let Some(p) = resp.interact_pointer_pos() {
                let frac = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                let new_secs = if have_total { total * frac } else { (pos + 1.0) * frac };
                return Some(new_secs.max(0.0));
            }
        }
        None
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
                    self.status = "Path does not exist or not a directory".into();
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
            self.player.playlist = vec![Track { path: file.clone(), title, artist: artist_final, album, album_dir, track_no: leading_no, disc_no }];
            self.player.index = 0;
            if let Err(e) = self.player.play_current() {
                self.status = format!("Play error: {e}");
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
            let cover_area_max = Vec2::new(460.0, 460.0);
            if let Some(tex) = &self.cover_tex {
                let size = tex.size();
                let ratio = (cover_area_max.x / size[0] as f32).min(cover_area_max.y / size[1] as f32).min(1.0);
                ui.image((tex.id(), egui::vec2(size[0] as f32 * ratio, size[1] as f32 * ratio)));
            } else {
                ui.label(RichText::new("No Cover").color(Color32::GRAY));
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
                let cover_area_max = Vec2::new(360.0, 360.0);
                if let Some(tex) = &self.cover_tex {
                    let size = tex.size();
                    let ratio = (cover_area_max.x / size[0] as f32).min(cover_area_max.y / size[1] as f32).min(1.0);
                    ui.image((tex.id(), egui::vec2(size[0] as f32 * ratio, size[1] as f32 * ratio)));
                } else {
                    ui.label(RichText::new("No Cover").color(Color32::GRAY));
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
                    self.status = format!("Play error: {e}");
                } else {
                    self.update_cover_from_current_track();
                }
            }
        });
    }

    fn bottom_controls(&mut self, ui: &mut egui::Ui) {
        let total = self.player.current_total_secs();
        let mut pos = self.player.current_pos().as_secs_f32();
        if total.is_finite() && pos > total {
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
                if let Some(new_secs) = self.seekbar_sized(ui, pos, total, seek_w, 14.0) {
                    if let Err(e) = self.player.seek_to_secs(new_secs) {
                        self.status = format!("Seek error: {e}");
                    }
                }
                ui.add_space(gap);
                ui.add_sized(
                    [time_w, row1_h],
                    egui::Label::new(RichText::new(seconds_to_mmss(if total.is_finite() { total } else { f32::NAN })).monospace()),
                );
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

                let prev_resp = icon_button_circle(ui, prev_d, "Previous Track", |p, r, c| draw_icon_prev(p, r, c));
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
                    "Resume"
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

                let next_resp = icon_button_circle(ui, next_d, "Next Track", |p, r, c| draw_icon_next(p, r, c));
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
                if ui
                    .add_sized([vol_w, 18.0], egui::Slider::new(&mut vol, 0.0..=2.0).text("Volume").show_value(false))
                    .changed() {
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
            UiView::Playlist => self.ui_page_playlist(ui),
        });

        let focused: bool = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        ctx.request_repaint_after(if focused { Duration::from_millis(60) } else { Duration::from_millis(800) });
    }
}

fn open_decoder(path: &Path) -> Result<Decoder<BufReader<File>>> {
    let f = File::open(path).with_context(|| format!("Cannot open file: {}", path.display()))?;
    let reader = BufReader::new(f);
    Decoder::new(reader).map_err(|e| anyhow!("rodio::Decoder error for {}: {e}", path.display()))
}

fn main() -> Result<()> {
    let renderer = match std::env::var("MUSA_RENDERER") {
        Ok(v) if v.eq_ignore_ascii_case("wgpu") => Renderer::Wgpu,
        _ => Renderer::Glow,
    };
    let mut native_opts = eframe::NativeOptions::default();
    native_opts.renderer = renderer;
    native_opts.viewport = egui::ViewportBuilder::default()
        .with_title("MUSA - Music Player")
        .with_app_id("musa")
        .with_inner_size([980.0, 720.0])
        .with_min_inner_size([760.0, 560.0]);
    if matches!(renderer, Renderer::Wgpu) {
        native_opts.wgpu_options.present_mode = eframe::egui_wgpu::wgpu::PresentMode::AutoNoVsync;
    }
    let app = MusaApp::new()?;
    eframe::run_native("MUSA - Music Player", native_opts, Box::new(|_| Box::new(app)))
        .map_err(|e| anyhow!("GUI error: {e}"))
}

