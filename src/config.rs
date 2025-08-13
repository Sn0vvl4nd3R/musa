use serde::{
    Serialize,
    Deserialize
};

use std::{
    fs,
    path::PathBuf
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowCfg {
    pub start_w: f32,
    pub start_h: f32,
    pub min_w: f32,
    pub min_h: f32,
    pub vsync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeCfg {
    pub bg_stops: [[u8; 3]; 3],
    pub accent: [u8; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiCfg {
    pub gap: f32,
    pub gap_small: f32,
    pub seek: SeekbarCfg,
    pub controls: ControlsCfg,
    pub volume: VolumeSliderCfg,
    pub bottom_bar_h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SeekbarCfg {
    pub time_width: f32,
    pub height: f32,
    pub knob_r: f32,
    pub knob_r_hover: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlsCfg {
    pub prev_d: f32,
    pub play_d: f32,
    pub next_d: f32,
    pub repeat_d: f32,
    pub shuffle_d: f32,
    pub row_min_h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VolumeSliderCfg {
    pub height: f32,
    pub knob_r: f32,
    pub knob_r_hover: f32,
    pub width_frac_right: f32,
    pub width_min: f32,
    pub width_max: f32,
    pub max_value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnimCfg {
    pub theme_ms: u64,
    pub bg: BgAnimCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VisualizerCfg {
    pub n_points: usize,
    pub band_h_ratio: f32,
    pub band_h_min: f32,
    pub band_h_max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerCfg {
    pub initial_volume: f32,
    pub volume_max: f32,
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

impl Default for WindowCfg {
    fn default() -> Self {
        Self {
            start_w: 900.0,
            start_h: 700.0,
            min_w: 780.0,
            min_h: 900.0,
            vsync: false,
        }
    }
}

impl Default for ThemeCfg {
    fn default() -> Self {
        Self {
            bg_stops: [[36, 36, 40], [24, 24, 28], [12, 12, 14]],
            accent: [120, 160, 255],
        }
    }
}

impl Default for UiCfg {
    fn default() -> Self {
        Self {
            gap: 8.0,
            gap_small: 6.0,
            seek: SeekbarCfg::default(),
            controls: ControlsCfg::default(),
            volume: VolumeSliderCfg::default(),
            bottom_bar_h: 84.0,
        }
    }
}

impl Default for SeekbarCfg {
    fn default() -> Self {
        Self {
            time_width: 68.0,
            height: 10.0,
            knob_r: 6.5,
            knob_r_hover: 7.5,
        }
    }
}

impl Default for ControlsCfg {
    fn default() -> Self {
        Self {
            prev_d: 40.0,
            play_d: 48.0,
            next_d: 40.0,
            repeat_d: 36.0,
            shuffle_d: 36.0,
            row_min_h: 28.0,
        }
    }
}

impl Default for VolumeSliderCfg {
    fn default() -> Self {
        Self {
            height: 8.0,
            knob_r: 4.0,
            knob_r_hover: 4.8,
            width_frac_right: 0.18,
            width_min: 90.0,
            width_max: 140.0,
            max_value: 2.0,
        }
    }
}

impl Default for AnimCfg {
    fn default() -> Self {
        Self {
            theme_ms: 420,
            bg: BgAnimCfg::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BgAnimCfg {
    pub enabled: bool,

    pub music_amount: f32,

    pub vis_gain: f32,

    pub depth_gain: f32,

    pub focus: f32,

    pub warp_base: f32,
    pub warp_music: f32,

    pub hue_follow: f32,

    pub lambert_gain: f32,
    pub rim_gain: f32,

    pub vol_base: f32,
    pub vol_music: f32,
}

impl Default for BgAnimCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            music_amount: 1.0,
            vis_gain: 1.28,
            depth_gain: 1.15,
            focus: 0.65,
            warp_base: 0.026,
            warp_music: 0.040,
            hue_follow: 0.035,
            lambert_gain: 0.80,
            rim_gain: 0.12,
            vol_base: 0.18,
            vol_music: 0.10,
        }
    }
}

impl Default for VisualizerCfg {
    fn default() -> Self {
        Self {
            n_points: 120,
            band_h_ratio: 0.28,
            band_h_min: 80.0,
            band_h_max: 200.0,
        }
    }
}

impl Default for PlayerCfg {
    fn default() -> Self {
        Self {
            initial_volume: 1.0,
            volume_max: 2.0,
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
