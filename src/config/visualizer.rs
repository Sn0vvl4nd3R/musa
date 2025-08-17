use serde::{
    Serialize,
    Deserialize
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VisualizerCfg {
    pub n_points: usize,
    pub band_h_ratio: f32,
    pub band_h_min: f32,
    pub band_h_max: f32,
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
