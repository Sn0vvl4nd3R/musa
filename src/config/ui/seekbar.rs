use serde::{
    Serialize,
    Deserialize
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SeekbarCfg {
    pub time_width: f32,
    pub height: f32,
    pub knob_r: f32,
    pub knob_r_hover: f32,
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
