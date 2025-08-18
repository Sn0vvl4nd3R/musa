use egui::Color32;
use anyhow::Result;
use super::anim::ThemeAnim;

use std::{
    sync::mpsc,
    path::PathBuf
};

use crate::{
    config::Config,
    player::Player
};

pub struct MusaApp {
    pub cfg: Config,
    pub player: Player,
    pub(super) view: super::types::UiView,
    pub(super) status: String,
    pub(super) current_dir: PathBuf,
    pub(super) current_dir_text: String,
    pub(super) dir_entries: Vec<super::types::DirEntryItem>,

    pub(super) cover_path: Option<PathBuf>,
    pub(super) cover_id_path: Option<String>,
    pub(super) cover_tex: Option<egui::TextureHandle>,
    pub(super) cover_rx: Option<mpsc::Receiver<anyhow::Result<(usize, usize, Vec<u8>, String, [Color32; 3])>>>,

    pub(super) bg_colors: [Color32; 3],
    pub(super) accent: Color32,
    pub(super) title_color: Color32,
    pub(super) header_color: Color32,

    pub(super) anim: ThemeAnim,

    pub(super) scan_rx: Option<mpsc::Receiver<anyhow::Result<Vec<crate::track::Track>>>>,

    pub(super) vis_draw: Vec<f32>,

    pub(super) vis_fast: Vec<f32>,
    pub(super) vis_slow: Vec<f32>,
    pub(super) vis_vals: Vec<f32>,
    pub(super) vis_tmp: Vec<f32>,
    pub(super) vis_pts: Vec<egui::Pos2>,

    pub(super) last_frame: std::time::Instant,
    pub(super) dt_sec: f32,

    pub(super) agc_env: f32,
    pub(super) agc_gain: f32,
    pub(super) agc_target: f32,
    pub(super) agc_gain_min: f32,
    pub(super) agc_gain_max: f32,

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
        super::pages::new_app(cfg)
    }
}
