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
    time::{
        Duration
    },
};

use crate::{
    player::Player,
    theme::{
        apply_visuals,
    },
    util::{
        seconds_to_mmss,
    },
    ui::widgets::{
        seekbar,
        draw_icon_play,
        draw_icon_prev,
        draw_icon_next,
        draw_icon_pause,
        icon_button_circle
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

    scan_rx: Option<mpsc::Receiver<anyhow::Result<Vec<crate::track::Track>>>>,

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
        pages::new_app()
    }
}

impl App for MusaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        anim::tick_theme_anim(self, ctx);

        let now = std::time::Instant::now();
        self.dt_sec = (now - self.last_frame).as_secs_f32().clamp(0.001, 0.05);
        self.last_frame = now;

        apply_visuals(ctx, self.accent);

        if self.bg_tex.is_none() {
            anim::rebuild_bg_texture(self, ctx);
        }
        anim::paint_bg_gradient(ctx, &self.bg_tex, self.bg_colors);

        cover::poll_cover_result(self, ctx);
        scanner::poll_scan_result(self, ctx);

        egui::TopBottomPanel::top("musa_top").frame(egui::Frame::none()).show(ctx, |ui| {
            ui.add_space(4.0);
            nav::nav_tabs(self, ui);
        });

        egui::TopBottomPanel::bottom("musa_bottom")
            .frame(egui::Frame::none())
            .resizable(false)
            .exact_height(84.0)
            .show(ctx, |ui| controls::bottom_controls(self, ui));

        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| match self.view {
            UiView::Browser => pages::browser::ui_page_browser(self, ui),
            UiView::Player => pages::player::ui_page_player(self, ui),
            UiView::Playlist => pages::playlist::ui_page_playlist(self, ui),
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
