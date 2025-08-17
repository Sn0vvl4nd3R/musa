use serde::{
    Serialize,
    Deserialize
};

pub mod volume;
pub mod seekbar;
pub mod controls;

pub use controls::ControlsCfg;
pub use seekbar::SeekbarCfg;
pub use volume::VolumeSliderCfg;

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
