use serde::{
    Serialize,
    Deserialize
};

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
