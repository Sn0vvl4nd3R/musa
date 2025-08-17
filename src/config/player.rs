use serde::{
    Serialize,
    Deserialize
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerCfg {
    pub initial_volume: f32,
    pub volume_max: f32,
}

impl Default for PlayerCfg {
    fn default() -> Self {
        Self {
            initial_volume: 1.0,
            volume_max: 2.0,
        }
    }
}
