use super::MusaApp;
use std::sync::OnceLock;
use crate::theme::lerp_srgb;
use egui::{
    self,
    Color32
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
fn rgb_to_hsv(c: Color32) -> (f32, f32, f32) {
    let r = c.r() as f32 / 255.0;
    let g = c.g() as f32 / 255.0;
    let b = c.b() as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d < 1e-6 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max <= 0.0 {
        0.0
    } else {
        d / max
    };
    (if h < 0.0 {
        h + 360.0
    } else {
        h
    }, s, max)
}

#[inline]
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color32::from_rgb(
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

#[inline]
fn base_gradient(v: f32, app: &MusaApp) -> Color32 {
    if v <= 0.5 {
        lerp_srgb(app.bg_colors[0], app.bg_colors[1], v * 2.0)
    } else {
        lerp_srgb(app.bg_colors[1], app.bg_colors[2], (v - 0.5) * 2.0)
    }
}

static START: OnceLock<std::time::Instant> = OnceLock::new();
#[inline]
fn time_now() -> f32 {
    let s = START.get_or_init(std::time::Instant::now);
    s.elapsed().as_secs_f32()
}

#[inline]
fn hash1(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

#[inline]
fn rand01(seed: u32) -> f32 {
    (hash1(seed) as f32) / (u32::MAX as f32)
}

#[derive(Clone, Copy)]
struct FlowParams {
    kx: [f32; 4],
    ky: [f32; 4],
    w: [f32; 4],
    ph: [f32; 4],
    warp: f32,
}

fn make_params(seed: u32) -> FlowParams {
    let mut kx = [0.0; 4];
    let mut ky = [0.0; 4];
    let mut w = [0.0; 4];
    let mut ph = [0.0; 4];

    for i in 0..4 {
        let si = seed ^ ((i as u32 + 1) * 0x9E37_79B9);
        let ang = rand01(si ^ 0x11) * std::f32::consts::TAU;
        let mag = 0.22 + 0.26 * rand01(si ^ 0x22);
        kx[i] = ang.cos() * mag;
        ky[i] = ang.sin() * mag;
        w[i] = 0.015 + 0.030 * rand01(si ^ 0x33);
        ph[i] = rand01(si ^ 0x44) * std::f32::consts::TAU;
    }

    FlowParams {
        kx,
        ky,
        w,
        ph,
        warp: 0.008 + 0.012 * rand01(seed ^ 0x55),
    }
}

#[inline]
fn flow_field(u: f32, v: f32, t: f32, p: &FlowParams) -> f32 {
    let du = (u * 0.9 + 0.6 * t).sin() * (v * 0.7 - 0.4 * t).cos() * p.warp;
    let dv = (u * 0.6 - 0.5 * t).cos() * (v * 0.8 + 0.3 * t).sin() * p.warp;
    let uu = u + du;
    let vv = v + dv;

    let mut acc = 0.0f32;
    let mut norm = 0.0f32;
    for i in 0..4 {
        let phase = (p.kx[i] * uu + p.ky[i] * vv) * std::f32::consts::TAU
            + (p.w[i] * t) * std::f32::consts::TAU
            + p.ph[i];
        let s = phase.sin();
        let a = [0.48, 0.36, 0.30, 0.24][i];
        acc += s * a;
        norm += a;
    }
    (acc / norm).clamp(-1.0, 1.0)
}

const VIS_GAIN: f32 = 1.28;
const DEPTH_GAIN: f32 = 1.15;
const FOCUS: f32 = 0.65;

#[inline]
fn shade(
    u: f32,
    v: f32,
    app: &MusaApp,
    p_main: &FlowParams,
    p_detail: &FlowParams,
    music: f32,
    t: f32,
) -> Color32 {
    let f1 = flow_field(u, v, t, p_main);
    let f2 = flow_field(u * 1.07 + 0.031, v * 1.03 + 0.047, t * 0.97, p_detail);
    let iso = (f1 * 0.6 + f2 * 0.4).clamp(-1.0, 1.0);

    let mus = (0.75 * music).clamp(0.0, 1.0);
    let warp_v = VIS_GAIN * (0.026 + 0.040 * mus);
    let vv = (v + iso * warp_v).clamp(0.0, 1.0);

    const EPS: f32 = 0.0035;
    let f1_px = flow_field(u + EPS, v, t, p_main);
    let f1_mx = flow_field(u - EPS, v, t, p_main);
    let f1_py = flow_field(u, v + EPS, t, p_main);
    let f1_my = flow_field(u, v - EPS, t, p_main);

    let f2_px = flow_field((u + EPS) * 1.07 + 0.031, v * 1.03 + 0.047, t * 0.97, p_detail);
    let f2_mx = flow_field((u - EPS) * 1.07 + 0.031, v * 1.03 + 0.047, t * 0.97, p_detail);
    let f2_py = flow_field(u * 1.07 + 0.031, (v + EPS) * 1.03 + 0.047, t * 0.97, p_detail);
    let f2_my = flow_field(u * 1.07 + 0.031, (v - EPS) * 1.03 + 0.047, t * 0.97, p_detail);

    let iso_x1 = f1_px * 0.6 + f2_px * 0.4;
    let iso_x0 = f1_mx * 0.6 + f2_mx * 0.4;
    let iso_y1 = f1_py * 0.6 + f2_py * 0.4;
    let iso_y0 = f1_my * 0.6 + f2_my * 0.4;

    let ddu = (iso_x1 - iso_x0) / (2.0 * EPS);
    let ddv = (iso_y1 - iso_y0) / (2.0 * EPS);

    let depth = DEPTH_GAIN * (0.85 + 0.30 * mus);
    let nx = -ddu * depth;
    let ny = -ddv * depth;
    let nz = 1.0;
    let inv = 1.0 / (nx * nx + ny * ny + nz * nz).sqrt();
    let (nx, ny, nz) = (nx * inv, ny * inv, nz * inv);

    let seed = (app.accent.r() as u32) * 73856093
        ^ (app.accent.g() as u32) * 19349663
        ^ (app.accent.b() as u32) * 83492791;
    let phi = 2.0 * std::f32::consts::PI * (0.03 * t + rand01(seed ^ 0xABCD));
    let light = (phi.cos() * 0.7, phi.sin() * 0.7, 0.72);

    let lambert = (nx * light.0 + ny * light.1 + nz * light.2).clamp(0.0, 1.0);
    let rim = (1.0 - nz).powf(2.0);

    let dx = u - 0.5;
    let dy = v - 0.5;
    let r = (dx * dx + dy * dy).sqrt();
    let center = smoothstep_inv(0.30, 0.85, r);
    let focus = center * FOCUS + (1.0 - center) * (1.0 - FOCUS);

    let c = base_gradient(vv, app);
    let (mut h, s, mut val) = rgb_to_hsv(c);

    let vol = VIS_GAIN * (0.18 + 0.10 * mus) * focus;
    val = (val + vol * (lambert * 0.85 + rim * 0.15 - 0.35)).clamp(0.0, 1.0);

    let (h_acc, _, _) = rgb_to_hsv(app.accent);
    let mut dh = h_acc - h;
    if dh > 180.0 {
        dh -= 360.0;
    }
    if dh < -180.0 {
        dh += 360.0;
    }
    h = (h + dh * 0.035 * focus).rem_euclid(360.0);

    hsv_to_rgb(h, s, val)
}

#[inline]
fn smoothstep_inv(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    1.0 - s
}

pub(super) fn paint_bg_gradient(ctx: &egui::Context, app: &MusaApp) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());

    let seed = (app.accent.r() as u32) * 73856093
        ^ (app.accent.g() as u32) * 19349663
        ^ (app.accent.b() as u32) * 83492791;
    let p_main = make_params(seed);
    let p_detail = make_params(seed ^ 0xDEAD_BEEF);
    let t = time_now();

    let music = {
        let raw = app.player.vis_buffer().take_recent(2048);
        if raw.len() >= 256 {
            let mut samp: Vec<f32> = raw.iter().step_by(8).map(|x| x.abs()).collect();
            samp.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let idx = ((samp.len() as f32) * 0.90).floor() as usize;
            let p90 = if samp.is_empty() {
                0.0
            } else {
                samp[idx.min(samp.len() - 1)]
            };
            (p90 * 1.6).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };

    let nx = ((rect.width() / 20.0).ceil() as usize).clamp(90, 220);
    let ny = ((rect.height() / 20.0).ceil() as usize).clamp(90, 220);

    let mut mesh = egui::Mesh::default();
    mesh.vertices.reserve((nx + 1) * (ny + 1));
    mesh.indices.reserve(nx * ny * 6);

    for j in 0..=ny {
        let v = j as f32 / ny as f32;
        let y = egui::lerp(rect.top()..=rect.bottom(), v);
        for i in 0..=nx {
            let u = i as f32 / nx as f32;
            let x = egui::lerp(rect.left()..=rect.right(), u);
            let col = shade(u, v, app, &p_main, &p_detail, music, t);
            mesh.colored_vertex(egui::pos2(x, y), col);
        }
    }

    let row = nx + 1;
    for j in 0..ny {
        for i in 0..nx {
            let i0 = (j * row + i) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + row as u32;
            let i3 = i2 + 1;
            mesh.add_triangle(i0, i2, i1);
            mesh.add_triangle(i1, i2, i3);
        }
    }

    painter.add(egui::Shape::mesh(mesh));
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

    app.anim.start = std::time::Instant::now();
    app.anim.dur = std::time::Duration::from_millis(420);
    app.anim.active = true;
}

pub(super) fn tick_theme_anim(app: &mut MusaApp, _ctx: &egui::Context) {
    if !app.anim.active {
        return;
    }
    let t = (std::time::Instant::now() - app.anim.start).as_secs_f32() / app.anim.dur.as_secs_f32();
    let k = ThemeAnim::ease(t);

    for i in 0..3 {
        app.bg_colors[i] = lerp_srgb(app.anim.from_bg[i], app.anim.to_bg[i], k);
    }
    app.accent = lerp_srgb(app.anim.from_accent, app.anim.to_accent, k);
    app.title_color = lerp_srgb(app.anim.from_title, app.anim.to_title, k);
    app.header_color = lerp_srgb(app.anim.from_header, app.anim.to_header, k);

    if t >= 1.0 {
        app.anim.active = false;
        app.bg_colors = app.anim.to_bg;
        app.accent = app.anim.to_accent;
        app.title_color = app.anim.to_title;
        app.header_color = app.anim.to_header;
    }
}
