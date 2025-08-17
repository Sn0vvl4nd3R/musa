use eframe::egui;

use serde::{
    Serialize,
    Deserialize
};

use std::{
    fs,
    path::PathBuf
};

pub mod ui;
pub mod anim;
pub mod theme;
pub mod window;
pub mod player;
pub mod visualizer;

pub use anim::AnimCfg;
pub use theme::ThemeCfg;
pub use window::WindowCfg;
pub use player::PlayerCfg;
pub use visualizer::VisualizerCfg;
pub use ui::{
    SeekbarCfg,
    UiCfg,
    VolumeSliderCfg
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub window: WindowCfg,
    pub theme: ThemeCfg,
    pub ui: UiCfg,
    pub anim: AnimCfg,
    pub visualizer: VisualizerCfg,
    pub player: PlayerCfg,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window: WindowCfg::default(),
            theme: ThemeCfg::default(),
            ui: UiCfg::default(),
            anim: AnimCfg::default(),
            visualizer: VisualizerCfg::default(),
            player: PlayerCfg::default(),
        }
    }
}

impl Config {
    pub fn default_path() -> PathBuf {
        if let Ok(p) = std::env::var("MUSA_CONFIG") {
            return PathBuf::from(p);
        }
        if let Some(dirs) = directories::ProjectDirs::from("rs", "Musa", "musa") {
            let mut p = dirs.config_dir().to_path_buf();
            p.push("config.toml");
            p
        } else {
            let mut p = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/"));
            p.push(".config/musa/config.toml");
            p
        }
    }

    pub fn load_or_create() -> anyhow::Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            let text = fs::read_to_string(&path)?;
            let mut cfg: Config = toml::from_str(&text)?;
            cfg.clamp();
            Ok(cfg)
        } else {
            let cfg = Config::default();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let text = toml::to_string_pretty(&cfg)?;
            fs::write(&path, text)?;
            Ok(cfg)
        }
    }

    fn clamp(&mut self) {
        self.ui.volume.max_value = self.ui.volume.max_value.max(0.1);
        self.player.initial_volume = self
            .player
            .initial_volume
            .clamp(0.0, self.ui.volume.max_value);
        self.visualizer.n_points = self.visualizer.n_points.max(16).min(4096);
    }

    pub fn accent_color32(&self) -> egui::Color32 {
        let [r, g, b] = self.theme.accent;
        egui::Color32::from_rgb(r, g, b)
    }

    pub fn bg_stops_color32(&self) -> [egui::Color32; 3] {
        let to = |rgb: [u8; 3]| egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        [to(self.theme.bg_stops[0]), to(self.theme.bg_stops[1]), to(self.theme.bg_stops[2])]
    }

    pub fn save_to_default(&self) -> anyhow::Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}
