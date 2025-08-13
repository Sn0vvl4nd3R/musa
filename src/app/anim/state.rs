use super::super::MusaApp;
use egui::{
    self,
    Color32
};
use crate::theme::{
    lerp_srgb,
    title_from_accent
};

pub struct ThemeAnim {
    pub active: bool,
    pub start: std::time::Instant,
    pub dur: std::time::Duration,

    pub from_bg: [Color32; 3],
    pub to_bg: [Color32; 3],

    pub from_accent: Color32,
    pub to_accent: Color32,

    pub from_title: Color32,
    pub to_title: Color32,

    pub from_header: Color32,
    pub to_header: Color32,
}

impl ThemeAnim {
    pub fn new() -> Self {
        Self {
            active: false,
            start: std::time::Instant::now(),
            dur: std::time::Duration::from_millis(420),
            from_bg: [Color32::BLACK; 3],
            to_bg: [Color32::BLACK; 3],
            from_accent: Color32::WHITE,
            to_accent: Color32::WHITE,
            from_title: Color32::WHITE,
            to_title: Color32::WHITE,
            from_header: Color32::WHITE,
            to_header: Color32::WHITE,
        }
    }

    #[inline]
    pub fn ease(t: f32) -> f32 {
        0.5 - 0.5 * (std::f32::consts::PI * t.clamp(0.0, 1.0)).cos()
    }
}

#[inline]
fn lerp_update(app: &mut MusaApp, k: f32) {
    for i in 0..3 {
        app.bg_colors[i] = lerp_srgb(app.anim.from_bg[i], app.anim.to_bg[i], k);
    }
    app.accent = lerp_srgb(app.anim.from_accent, app.anim.to_accent, k);
    app.title_color = lerp_srgb(app.anim.from_title, app.anim.to_title,  k);
    app.header_color = lerp_srgb(app.anim.from_header, app.anim.to_header, k);
}

pub fn begin_theme_anim(app: &mut MusaApp, to_bg: [Color32; 3], to_accent: Color32) {
    app.anim.from_bg = app.bg_colors;
    app.anim.to_bg = to_bg;

    app.anim.from_accent = app.accent;
    app.anim.to_accent = to_accent;

    app.anim.from_title  = app.title_color;
    app.anim.to_title = title_from_accent(to_accent);

    app.anim.from_header = app.header_color;
    app.anim.to_header = title_from_accent(to_accent);

    app.anim.start = std::time::Instant::now();
    app.anim.dur = std::time::Duration::from_millis(app.cfg.anim.theme_ms);
    app.anim.active = true;
}

pub fn tick_theme_anim(app: &mut MusaApp, _ctx: &egui::Context) {
    if !app.anim.active {
        return;
    }
    let t = (std::time::Instant::now() - app.anim.start).as_secs_f32() / app.anim.dur.as_secs_f32();
    let k = ThemeAnim::ease(t);
    lerp_update(app, k);

    if t >= 1.0 {
        app.anim.active = false;
        app.bg_colors = app.anim.to_bg;
        app.accent = app.anim.to_accent;
        app.title_color = app.anim.to_title;
        app.header_color = app.anim.to_header;
    }
}
