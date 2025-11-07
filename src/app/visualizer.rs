use egui;

const EPSILON: f32 = 1e-4;
const HORIZON: usize = 6_144;

#[inline(always)]
fn catmull_rom(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;

    0.5 * ((2.0 * y1)
        + (-y0 + y2) * t
        + (2.0 * y0 - 5.0 * y1 + 4.0 * y2 - y3) * t2
        + (-y0 + 3.0 * y1 - 3.0 * y2 + y3) * t3)
}

#[inline(always)]
fn catmull_rom_clamped(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let local_min = y0.min(y1).min(y2).min(y3);
    let local_max = y0.max(y1).max(y2).max(y3);
    catmull_rom(y0, y1, y2, y3, t).clamp(local_min, local_max)
}

#[inline(always)]
fn p90_in_place(buf: &mut [f32]) -> f32 {
    let len = buf.len();
    if len == 0 {
        return EPSILON;
    }

    let k = ((len as f32) * 0.90).floor().clamp(0.0, (len - 1) as f32) as usize;

    let (_, pivot, _) = buf.select_nth_unstable_by(k, |a, b| a.total_cmp(b));
    let v = *pivot;

    if v.is_finite() {
        v.max(EPSILON)
    } else {
        EPSILON
    }
}

pub(crate) fn draw_visualizer_bg(app: &mut super::MusaApp, ui: &mut egui::Ui, dt: f32) {
    let full = ui.max_rect();

    let cfgv = &app.cfg.visualizer;
    let mut band_h = full.height() * cfgv.band_h_ratio;
    band_h = band_h.clamp(cfgv.band_h_min, cfgv.band_h_max);

    if band_h < 40.0 {
        return;
    }

    let band = egui::Rect::from_min_max(
        egui::pos2(full.left() + 12.0, full.bottom() - band_h - 12.0),
        egui::pos2(full.right() - 12.0, full.bottom() - 12.0),
    );

    let raw = app.player.vis_buffer().take_recent(HORIZON);
    let raw_len = raw.len();
    if raw_len < 512 {
        return;
    }

    let n = app.vis_draw.len().max(120);
    if app.vis_draw.len() != n {
        app.vis_draw.resize(n, 0.0);
    }

    let fast_w = ((raw_len as f32) / (n as f32 * 1.5)).clamp(32.0, 256.0) as usize;
    if raw_len < fast_w {
        return;
    }

    let slow_w = (fast_w * 4).clamp(fast_w + 8, 2_048);
    let hop = ((fast_w as f32) * 0.50).max(12.0) as usize;

    app.vis_fast.clear();
    app.vis_slow.clear();
    app.vis_vals.clear();
    app.vis_tmp.clear();
    app.vis_pts.clear();

    app.vis_tmp.resize(raw_len + 1, 0.0);
    {
        let s2 = &mut app.vis_tmp[..];
        let mut acc = 0.0f32;
        s2[0] = 0.0;

        for (i, &x) in raw.iter().enumerate() {
            acc += x * x;
            s2[i + 1] = acc;
        }
    }
    let s2 = &app.vis_tmp[..];

    let inv_fast_w = 1.0 / fast_w as f32;

    let mut idx = raw_len;
    while app.vis_fast.len() < n && idx >= fast_w {
        let start_f = idx - fast_w;
        let sumsq_f = s2[idx] - s2[start_f];
        let rms_f = (sumsq_f * inv_fast_w).sqrt();

        let mut peak = 0.0f32;
        for &x in &raw[start_f..idx] {
            let a = x.abs();
            if a > peak {
                peak = a;
            }
        }

        app.vis_fast.push(rms_f * 0.65 + peak * 0.35);

        let start_s = idx.saturating_sub(slow_w);
        let len_s = idx - start_s;
        let inv_len_s = 1.0 / len_s as f32;
        let sumsq_s = s2[idx] - s2[start_s];
        app.vis_slow.push((sumsq_s * inv_len_s).sqrt());

        idx = idx.saturating_sub(hop);
    }

    if app.vis_fast.len() < 8 {
        return;
    }

    app.vis_fast.reverse();
    app.vis_slow.reverse();

    let m = app.vis_fast.len();

    app.vis_vals.resize(m, 0.0);
    for i in 0..m {
        let fast = app.vis_fast[i];
        let slow = app.vis_slow[i];
        let onset = (fast - slow).max(0.0);
        app.vis_vals[i] = fast * 0.75 + onset * 0.60;
    }

    app.vis_tmp.clear();
    app.vis_tmp.extend_from_slice(&app.vis_vals);
    let p90 = p90_in_place(&mut app.vis_tmp);

    let alpha_env = 1.0 - (-dt / 0.80).exp();
    app.agc_env += (p90 - app.agc_env) * alpha_env;

    let env = app.agc_env.max(EPSILON);
    let g_tgt = (app.agc_target / env).clamp(app.agc_gain_min, app.agc_gain_max);

    let alpha_gain = 1.0 - (-dt / 0.25).exp();
    app.agc_gain += (g_tgt - app.agc_gain) * alpha_gain;

    let knee = 1.8f32;
    let inv_norm = 1.0 / knee.tanh();
    for v in &mut app.vis_vals {
        let x = *v * app.agc_gain;
        *v = (knee * x).tanh() * inv_norm;
    }

    app.vis_tmp.resize(n, 0.0);
    {
        let tgt = &mut app.vis_tmp[..];

        if m == 1 {
            let v = app.vis_vals[0];
            for y in tgt.iter_mut() {
                *y = v;
            }
        } else {
            let scale = (m - 1) as f32 / (n - 1) as f32;
            let last = (m - 1) as isize;

            for k in 0..n {
                let u = (k as f32) * scale;
                let i1 = u.floor() as isize;
                let t = u - (i1 as f32);

                let i0 = (i1 - 1).clamp(0, last) as usize;
                let i1u = i1.clamp(0, last) as usize;
                let i2 = (i1 + 1).clamp(0, last) as usize;
                let i3 = (i1 + 2).clamp(0, last) as usize;

                let y0 = app.vis_vals[i0];
                let y1 = app.vis_vals[i1u];
                let y2 = app.vis_vals[i2];
                let y3 = app.vis_vals[i3];

                tgt[k] = catmull_rom_clamped(y0, y1, y2, y3, t);
            }
        }

        for _ in 0..2 {
            if n <= 2 {
                break;
            }

            let mut prev = tgt[0];
            for i in 1..n - 1 {
                let cur = tgt[i];
                let nxt = tgt[i + 1];
                tgt[i] = 0.25 * prev + 0.5 * cur + 0.25 * nxt;
                prev = cur;
            }
        }
    }

    let tau_up = 0.060; // атака ~60 ms
    let tau_dn = 0.180; // спад ~180 ms
    let a_up = 1.0 - (-dt / tau_up).exp();
    let a_dn = 1.0 - (-dt / tau_dn).exp();

    {
        let tgt = &app.vis_tmp[..];
        let draw = &mut app.vis_draw[..];

        for (i, &des) in tgt.iter().enumerate().take(n) {
            let cur = draw[i];
            let a = if des > cur { a_up } else { a_dn };
            draw[i] = cur + (des - cur) * a;
        }
    }

    if n > 2 {
        let draw = &mut app.vis_draw[..];
        let mut prev = draw[0];
        for i in 1..n - 1 {
            let cur = draw[i];
            let nxt = draw[i + 1];
            draw[i] = (prev + 2.0 * cur + nxt) * 0.25;
            prev = cur;
        }
    }

    let painter = ui.painter_at(band);

    let h = band.height() * 1.02;
    let baseline = band.bottom() - 4.0;
    if n < 2 {
        return;
    }
    let step = band.width() / (n as f32 - 1.0);

    app.vis_pts.clear();
    app.vis_pts.reserve(n);

    let left = band.left();
    let top = band.top();

    for (i, &v) in app.vis_draw.iter().enumerate() {
        let x = left + (i as f32) * step;
        let mut y = baseline - h * v;
        if y < top {
            y = top;
        } else if y > baseline {
            y = baseline;
        }
        app.vis_pts.push(egui::pos2(x, y));
    }

    let glow = egui::Stroke::new(
        6.0,
        egui::Color32::from_rgba_unmultiplied(app.accent.r(), app.accent.g(), app.accent.b(), 42),
    );
    painter.add(egui::Shape::line(app.vis_pts.clone(), glow));

    let contour = egui::Stroke::new(
        1.8,
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 170),
    );
    painter.add(egui::Shape::line(std::mem::take(&mut app.vis_pts), contour));
}
