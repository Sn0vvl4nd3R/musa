use super::super::MusaApp;
use egui::{
    self,
    Color32
};
use super::{
    time_rng::{
        rand01,
        time_now
    },
    flow::{
        FlowParams,
        flow_field,
        make_params,
    },
    color::{
        rgb_to_hsv,
        hsv_to_rgb,
        base_gradient,
    }
};

const VIS_GAIN: f32 = 1.28;
const DEPTH_GAIN: f32 = 1.15;
const FOCUS: f32 = 0.65;

#[inline]
fn smoothstep_inv(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    1.0 - s
}

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

pub fn paint_bg_gradient(ctx: &egui::Context, app: &MusaApp) {
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
