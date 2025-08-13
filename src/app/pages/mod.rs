use super::*;
use crate::util::home_dir;
use crate::config::Config;

pub mod settings;
pub mod browser;
pub mod player;
pub mod playlist;

pub(super) fn new_app(cfg: Config) -> anyhow::Result<super::MusaApp> {
    let start_dir = home_dir();

    let accent = cfg.accent_color32();
    let bg_init = crate::theme::make_gradient_stops(cfg.bg_stops_color32());
    let vis_points = cfg.visualizer.n_points;
    let init_volume = cfg.player.initial_volume;

    let mut player = crate::player::Player::new();
    player.volume = init_volume;

    Ok(super::MusaApp {
        cfg,
        player,

        view: super::UiView::Player,
        status: String::new(),
        current_dir: start_dir.clone(),
        current_dir_text: start_dir.display().to_string(),
        dir_entries: super::scanner::read_dir_items(&start_dir),

        cover_path: None,
        cover_id_path: None,
        cover_tex: None,
        cover_rx: None,

        bg_colors: bg_init,
        accent,
        title_color: crate::theme::title_from_accent(accent),
        header_color: crate::theme::title_from_accent(accent),

        anim: super::anim::ThemeAnim::new(),

        scan_rx: None,

        vis_draw: vec![0.0; vis_points],
        vis_fast: Vec::new(),
        vis_slow: Vec::new(),
        vis_vals: Vec::new(),
        vis_tmp: Vec::new(),
        vis_pts: Vec::new(),

        last_frame: std::time::Instant::now(),
        dt_sec: 1.0 / 60.0,

        agc_env: 0.05,
        agc_gain: 1.0,
        agc_target: 0.55,
        agc_gain_min: 0.8,
        agc_gain_max: 3.0,

        bg_time: 0.0,

        bg_music_drive: 0.0,
        bg_music_env: 0.0,
        bg_pulse: 0.0,

        bg_phase1: 0.0,
        bg_phase2: 0.35,
        bg_phase3: 0.62,
        bg_phase4: 0.17,
        bg_speed1: 0.010,
        bg_speed2: 0.008,
        bg_speed3: 0.006,
        bg_speed4: 0.007,
    })
}
