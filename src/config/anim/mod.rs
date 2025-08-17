use serde::{
    Serialize,
    Deserialize
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnimCfg {
    pub theme_ms: u64,
    pub bg: BgAnimCfg,
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
