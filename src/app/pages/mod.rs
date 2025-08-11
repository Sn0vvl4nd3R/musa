use super::*;
use egui::Color32;
use crate::util::home_dir;

pub mod player;
pub mod browser;
pub mod playlist;

pub(super) fn new_app() -> anyhow::Result<super::MusaApp> {
    let start_dir = home_dir();
    Ok(super::MusaApp {
        player: crate::player::Player::new(),
        view: super::UiView::Player,
        status: String::new(),
        current_dir: start_dir.clone(),
        current_dir_text: start_dir.display().to_string(),
        dir_entries: super::scanner::read_dir_items(&start_dir),

        cover_path: None,
        cover_id_path: None,
        cover_tex: None,
        cover_rx: None,

        bg_colors: crate::theme::make_gradient_stops([
            Color32::from_rgb(36,36,40),
            Color32::from_rgb(24,24,28),
            Color32::from_rgb(12,12,14),
        ]),
        bg_tex: None,
        accent: Color32::from_rgb(120,160,255),
        title_color: crate::theme::title_from_accent(Color32::from_rgb(120,160,255)),
        header_color: crate::theme::title_from_accent(Color32::from_rgb(120,160,255)),

        anim: super::anim::ThemeAnim::new(),

        scan_rx: None,

        vis_draw: vec![0.0; 120],
        last_frame: std::time::Instant::now(),
        dt_sec: 1.0/60.0,

        agc_env: 0.05,
        agc_gain: 1.0,
        agc_target: 0.55,
        agc_gain_min: 0.8,
        agc_gain_max: 3.0,
    })
}
