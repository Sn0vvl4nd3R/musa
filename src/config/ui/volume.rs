use serde::{
    Serialize,
    Deserialize
};

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
