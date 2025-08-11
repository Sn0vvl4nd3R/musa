use super::MusaApp;
use crate::theme::lerp_srgb;
use egui::{
    self,
    Color32,
};
use std::time::{
    Instant,
    Duration,
};


pub struct ThemeAnim {
    pub active: bool,
    pub start: Instant,
    pub dur: Duration,

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
            start: Instant::now(),
            dur: Duration::from_millis(420),
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

pub(super) fn rebuild_bg_texture(app: &mut MusaApp, ctx: &egui::Context) {
    let n = 1024usize;
    let mut img = egui::ColorImage::new([1, n], app.bg_colors[1]);
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let c = if t <= 0.5 {
            let tt = t * 2.0;
            lerp_srgb(app.bg_colors[0], app.bg_colors[1], tt)
        } else {
            let tt = (t - 0.5) * 2.0;
            lerp_srgb(app.bg_colors[1], app.bg_colors[2], tt)
        };
        img.pixels[i] = c;
    }
    if let Some(tex) = &mut app.bg_tex {
        tex.set(img, egui::TextureOptions::LINEAR);
    } else {
        app.bg_tex = Some(ctx.load_texture("bg_gradient", img, egui::TextureOptions::LINEAR));
    }
}

pub(super) fn paint_bg_gradient(ctx: &egui::Context, tex: &Option<egui::TextureHandle>, fallback: [Color32; 3]) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    if let Some(t) = tex {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(t.id(), rect, uv, Color32::WHITE);
    } else {
        painter.rect_filled(rect, 0.0, fallback[1]);
    }
}

pub(super) fn begin_theme_anim(app: &mut MusaApp, to_bg: [Color32; 3], to_accent: Color32) {
    app.anim.from_bg = app.bg_colors;
    app.anim.to_bg = to_bg;

    app.anim.from_accent = app.accent;
    app.anim.to_accent = to_accent;

    app.anim.from_title = app.title_color;
    app.anim.to_title = crate::theme::title_from_accent(to_accent);

    app.anim.from_header = app.header_color;
    app.anim.to_header = crate::theme::title_from_accent(to_accent);

    app.anim.start = Instant::now();
    app.anim.dur = Duration::from_millis(420);
    app.anim.active = true;
}

pub(super) fn tick_theme_anim(app: &mut MusaApp, ctx: &egui::Context) {
    if !app.anim.active {
        return;
    }
    let t = (Instant::now() - app.anim.start).as_secs_f32() / app.anim.dur.as_secs_f32();
    let k = ThemeAnim::ease(t);

    for i in 0..3 {
        app.bg_colors[i] = lerp_srgb(app.anim.from_bg[i], app.anim.to_bg[i], k);
    }
    app.accent = lerp_srgb(app.anim.from_accent, app.anim.to_accent, k);
    app.title_color = lerp_srgb(app.anim.from_title, app.anim.to_title, k);
    app.header_color = lerp_srgb(app.anim.from_header, app.anim.to_header, k);

    rebuild_bg_texture(app, ctx);

    if t >= 1.0 {
        app.anim.active = false;
        app.bg_colors = app.anim.to_bg;
        app.accent = app.anim.to_accent;
        app.title_color = app.anim.to_title;
        app.header_color = app.anim.to_header;
        rebuild_bg_texture(app, ctx);
    }
}
