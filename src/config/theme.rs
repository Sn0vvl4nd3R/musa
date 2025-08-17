use serde::{
    Serialize,
    Deserialize
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeCfg {
    pub bg_stops: [[u8; 3]; 3],
    pub accent: [u8; 3],
}

impl Default for ThemeCfg {
    fn default() -> Self {
        Self {
            bg_stops: [[36, 36, 40], [24, 24, 28], [12, 12, 14]],
            accent: [120, 160, 255],
        }
    }
}
