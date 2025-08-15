use egui::Color32;
use anyhow::Result;
use anim::ThemeAnim;

use eframe::{
    App,
    Frame
};

use std::{
    sync::mpsc,
    path::PathBuf,
    time::Duration,
};

use crate::{
    player::Player,
    config::Config,
    theme::apply_visuals,
    ui::widgets::{
        draw_icon_play,
        draw_icon_prev,
        draw_icon_next,
        draw_icon_pause,
    },
};

mod types;
mod anim;
mod visualizer;
mod nav;
mod controls;
mod cover;
mod scanner;
mod pages;
mod color;

pub use types::{
    UiView,
    DirEntryItem
};

pub struct MusaApp {
    pub cfg: Config,
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
    accent: Color32,
    title_color: Color32,
    header_color: Color32,

    anim: ThemeAnim,

    scan_rx: Option<mpsc::Receiver<anyhow::Result<Vec<crate::track::Track>>>>,

    vis_draw: Vec<f32>,

    vis_fast: Vec<f32>,
    vis_slow: Vec<f32>,
    vis_vals: Vec<f32>,
    vis_tmp: Vec<f32>,
    vis_pts: Vec<egui::Pos2>,

    last_frame: std::time::Instant,
    dt_sec: f32,

    agc_env: f32,
    agc_gain: f32,
    agc_target: f32,
    agc_gain_min: f32,
    agc_gain_max: f32,

    pub bg_time: f32,

    pub bg_music_drive: f32,
    pub bg_music_env: f32,
    pub bg_pulse: f32,

    pub bg_phase1: f32,
    pub bg_phase2: f32,
    pub bg_phase3: f32,
    pub bg_phase4: f32,
    pub bg_speed1: f32,
    pub bg_speed2: f32,
    pub bg_speed3: f32,
    pub bg_speed4: f32,
}

impl MusaApp {
    pub fn new(cfg: Config) -> Result<Self> {
        pages::new_app(cfg)
    }
}

impl App for MusaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let now = std::time::Instant::now();

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.player.toggle_pause();
            }
            if i.key_pressed(egui::Key::ArrowLeft) {
                if let Err(e) = self.player.prev() {
                    self.status = format!("Prev error: {e}");
                }
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                if let Err(e) = self.player.next() {
                    self.status = format!("Next error: {e}");
                }
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                let maxv = self.cfg.ui.volume.max_value;
                self.player.set_volume((self.player.volume + 0.05).min(maxv));
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                self.player.set_volume((self.player.volume - 0.05).max(0.0));
            }
        });

        self.dt_sec = (now - self.last_frame).as_secs_f32().clamp(0.001, 0.05);
        self.last_frame = now;
        self.bg_time += self.dt_sec;

        let recent = self.player.vis_buffer().take_recent(4096);
        let target = if recent.len() >= 256 {
            let mut v: Vec<f32> = recent.iter().map(|x| x.abs()).collect();
            v.sort_by(|a,b| a.partial_cmp(b).unwrap());
            let p = v[((v.len() as f32 * 0.90) as usize).min(v.len() - 1)];
            (p * 2.0).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let a_fast = 1.0 - (-self.dt_sec / 0.15).exp();
        self.bg_music_drive += (target - self.bg_music_drive) * a_fast;

        let a_slow = 1.0 - (-self.dt_sec / 0.90).exp();
        self.bg_music_env += (target - self.bg_music_env) * a_slow;

        let onset = (self.bg_music_drive - self.bg_music_env * 1.06).max(0.0);
        self.bg_pulse = (self.bg_pulse + onset * 2.4).min(1.0);
        self.bg_pulse *= ( -self.dt_sec / 0.45 ).exp();

        let base = [0.010, 0.008, 0.006, 0.007];
        let speed_mul = 0.7 + 0.9 * self.bg_music_env;

        let targets = [
            base[0] * speed_mul,
            base[1] * speed_mul,
            base[2] * speed_mul,
            base[3] * speed_mul,
        ];
        let a_v = 1.0 - (-self.dt_sec / 0.70).exp();
        self.bg_speed1 += (targets[0] - self.bg_speed1) * a_v;
        self.bg_speed2 += (targets[1] - self.bg_speed2) * a_v;
        self.bg_speed3 += (targets[2] - self.bg_speed3) * a_v;
        self.bg_speed4 += (targets[3] - self.bg_speed4) * a_v;

        self.bg_phase1 = (self.bg_phase1 + self.bg_speed1 * self.dt_sec) % 1.0;
        self.bg_phase2 = (self.bg_phase2 + self.bg_speed2 * self.dt_sec) % 1.0;
        self.bg_phase3 = (self.bg_phase3 + self.bg_speed3 * self.dt_sec) % 1.0;
        self.bg_phase4 = (self.bg_phase4 + self.bg_speed4 * self.dt_sec) % 1.0;

        anim::tick_theme_anim(self, ctx);
        apply_visuals(ctx, self.accent);

        anim::paint_bg_gradient(ctx, self);

        cover::poll_cover_result(self, ctx);
        scanner::poll_scan_result(self, ctx);

        egui::TopBottomPanel::top("musa_top").frame(egui::Frame::none()).show(ctx, |ui| {
            ui.add_space(4.0);
            nav::nav_tabs(self, ui);
        });

        egui::TopBottomPanel::bottom("musa_bottom")
            .frame(egui::Frame::none())
            .resizable(false)
            .exact_height(self.cfg.ui.bottom_bar_h)
            .show(ctx, |ui| controls::bottom_controls(self, ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| match self.view {
            UiView::Settings => pages::settings::ui_page_settings(self, ui),
            UiView::Browser => pages::browser::ui_page_browser(self, ui),
            UiView::Player => pages::player::ui_page_player(self, ui),
            UiView::Playlist => pages::playlist::ui_page_playlist(self, ui),
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
