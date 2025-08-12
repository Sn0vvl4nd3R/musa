use egui;

#[inline]
fn catmull_rom(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * y1)
        + (-y0 + y2) * t
        + (2.0 * y0 - 5.0 * y1 + 4.0 * y2 - y3) * t2
        + (-y0 + 3.0 * y1 - 3.0 * y2 + y3) * t3)
}

#[inline]
fn p90_in_place(buf: &mut [f32]) -> f32 {
    let len = buf.len();
    if len == 0 {
        return 1e-4;
    }
    let k = ((len as f32) * 0.90).floor()
        .clamp(0.0, (len - 1) as f32) as usize;

    let (_, pivot, _) = buf.select_nth_unstable_by(k, |a, b| a.total_cmp(b));
    let v = *pivot;
    if v.is_finite() {
        v.max(1e-4)
    } else {
        1e-4
    }
}

pub(crate) fn draw_visualizer_bg(app: &mut super::MusaApp, ui: &mut egui::Ui, dt: f32) {
    let full = ui.max_rect();

    let band_h = (full.height() * 0.28).clamp(80.0, 200.0);
    let band = egui::Rect::from_min_max(
        egui::pos2(full.left() + 12.0, full.bottom() - band_h - 12.0),
        egui::pos2(full.right() - 12.0, full.bottom() - 12.0),
    );
    if band.height() < 40.0 {
        return;
    }

    let horizon = 6_144;
    let raw = app.player.vis_buffer().take_recent(horizon);
    if raw.len() < 512 {
        return;
    }

    let n = app.vis_draw.len().max(120);
    if app.vis_draw.len() != n {
        app.vis_draw.resize(n, 0.0);
    }

    let fast_w = ((raw.len() as f32 / (n as f32 * 1.5)).clamp(32.0, 256.0)) as usize;
    let slow_w = (fast_w * 4).clamp(fast_w + 8, 2048);
    let hop = ((fast_w as f32) * 0.50).max(12.0) as usize;

    app.vis_fast.clear();
    app.vis_slow.clear();
    app.vis_vals.clear();
    app.vis_tmp.clear();
    app.vis_pts.clear();

    app.vis_fast.reserve(n);
    app.vis_slow.reserve(n);
    app.vis_vals.reserve(n);
    app.vis_tmp.reserve(raw.len().max(n));
    app.vis_pts.reserve(n);

    app.vis_tmp.resize(raw.len() + 1, 0.0);
    for i in 0..raw.len() {
        app.vis_tmp[i + 1] = app.vis_tmp[i] + raw[i] * raw[i];
    }
    let s2 = &app.vis_tmp;

    let mut idx = raw.len();
    while app.vis_fast.len() < n && idx >= fast_w {
        let start_f = idx - fast_w;
        let sumsq_f = s2[idx] - s2[start_f];
        let rms_f = (sumsq_f / fast_w as f32).sqrt();

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
        let sumsq_s = s2[idx] - s2[start_s];
        app.vis_slow.push((sumsq_s / (len_s as f32)).sqrt());

        idx = idx.saturating_sub(hop);
    }
    if app.vis_fast.len() < 8 {
        return;
    }
    app.vis_fast.reverse();
    app.vis_slow.reverse();

    let m = app.vis_fast.len();
    for i in 0..m {
        let onset = (app.vis_fast[i] - app.vis_slow[i]).max(0.0);
        app.vis_vals.push(app.vis_fast[i] * 0.75 + onset * 0.60);
    }

    app.vis_tmp.clear();
    app.vis_tmp.extend_from_slice(&app.vis_vals);
    let p90 = p90_in_place(&mut app.vis_tmp);

    let alpha_env  = 1.0 - (-dt / 0.80).exp();
    app.agc_env += (p90 - app.agc_env) * alpha_env;

    let g_tgt = (app.agc_target / app.agc_env).clamp(app.agc_gain_min, app.agc_gain_max);
    let alpha_gain = 1.0 - (-dt / 0.25).exp();
    app.agc_gain += (g_tgt - app.agc_gain) * alpha_gain;

    for v in &mut app.vis_vals {
        *v *= app.agc_gain;
    }

    let knee = 1.8f32;
    let inv_norm = 1.0 / knee.tanh();
    for v in &mut app.vis_vals {
        *v = (knee * *v).tanh() * inv_norm;
    }

    app.vis_tmp.resize(n, 0.0);
    for k in 0..n {
        let u  = (k as f32) * ((m - 1) as f32) / ((n - 1) as f32);
        let i1 = u.floor() as isize;
        let t  = u - (i1 as f32);

        let i0 = (i1 - 1).clamp(0, (m - 1) as isize) as usize;
        let i1u = i1.clamp(0, (m - 1) as isize) as usize;
        let i2 = (i1 + 1).clamp(0, (m - 1) as isize) as usize;
        let i3 = (i1 + 2).clamp(0, (m - 1) as isize) as usize;

        app.vis_tmp[k] = catmull_rom(
            app.vis_vals[i0], app.vis_vals[i1u], app.vis_vals[i2], app.vis_vals[i3], t
        );
    }
    let tgt = &mut app.vis_tmp;

    for _ in 0..2 {
        let mut prev = tgt[0];
        for i in 1..n - 1 {
            let cur = tgt[i];
            let nxt = tgt[i + 1];
            tgt[i] = 0.25 * prev + 0.5 * cur + 0.25 * nxt;
            prev = cur;
        }
    }

    let tau_up = 0.040;
    let tau_dn = 0.120;
    let a_up = 1.0 - (-dt / tau_up).exp();
    let a_dn = 1.0 - (-dt / tau_dn).exp();

    for i in 0..n {
        let cur = app.vis_draw[i];
        let des = tgt[i];
        let a = if des > cur { a_up } else { a_dn };
        app.vis_draw[i] = cur + (des - cur) * a;
    }

    for i in 1..n - 1 {
        app.vis_draw[i] = (app.vis_draw[i - 1] + 2.0 * app.vis_draw[i] + app.vis_draw[i + 1]) * 0.25;
    }

    let painter = ui.painter_at(band);
    let h = band.height() * 1.02;
    let baseline = band.bottom() - 4.0;
    let step = band.width() / (n as f32 - 1.0);

    app.vis_pts.clear();
    for i in 0..n {
        let x = band.left() + i as f32 * step;
        let y = (baseline - h * app.vis_draw[i]).clamp(band.top(), baseline);
        app.vis_pts.push(egui::pos2(x, y));
    }

    let glow = egui::Stroke::new(
        6.0,
        egui::Color32::from_rgba_unmultiplied(app.accent.r(), app.accent.g(), app.accent.b(), 42),
    );
    painter.add(egui::Shape::line(app.vis_pts.clone(), glow));

    let contour = egui::Stroke::new(1.8, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 170));
    painter.add(egui::Shape::line(std::mem::take(&mut app.vis_pts), contour));
}
