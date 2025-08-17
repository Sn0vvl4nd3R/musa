use serde::{
    Serialize,
    Deserialize
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowCfg {
    pub start_w: f32,
    pub start_h: f32,
    pub min_w: f32,
    pub min_h: f32,
    pub vsync: bool,
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
