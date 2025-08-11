use egui;

#[inline]
fn soft_sat(x: f32, knee: f32) -> f32 {
    (knee * x).tanh() / knee.tanh()
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

    let mut fast: Vec<f32> = Vec::with_capacity(n);
    let mut slow: Vec<f32> = Vec::with_capacity(n);
    let mut idx = raw.len().saturating_sub(fast_w);
    while fast.len() < n && idx >= fast_w {
        let seg_f = &raw[idx - fast_w .. idx];
        let mut sum = 0.0f32;
        let mut peak = 0.0f32;
        for &x in seg_f {
            sum += x*x;
            peak = peak.max(x.abs());
        }
        let rms_f = (sum / fast_w as f32).sqrt();
        fast.push(rms_f * 0.65 + peak * 0.35);

        let start_s = idx.saturating_sub(slow_w);
        let seg_s = &raw[start_s..idx];
        let mut sum2 = 0.0f32;
        for &x in seg_s {
            sum2 += x*x;
        }
        slow.push((sum2 / seg_s.len() as f32).sqrt());

        idx = idx.saturating_sub(hop);
    }
    if fast.len() < 8 {
        return;
    }
    fast.reverse();
    slow.reverse();

    let mut vals: Vec<f32> = Vec::with_capacity(fast.len());
    for i in 0..fast.len() {
        let onset = (fast[i] - slow[i]).max(0.0);
        vals.push(fast[i] * 0.75 + onset * 0.60);
    }

    let mut tmp = vals.clone();
    tmp.sort_by(|a,b| a.partial_cmp(b).unwrap());
    let p90 = tmp[(tmp.len() as f32 * 0.90).floor() as usize].max(1e-4);

    let alpha_env = 1.0 - (-dt / 0.80).exp();
    app.agc_env = app.agc_env + (p90 - app.agc_env) * alpha_env;

    let g_tgt = (app.agc_target / app.agc_env).clamp(app.agc_gain_min, app.agc_gain_max);

    let alpha_gain = 1.0 - (-dt / 0.25).exp();
    app.agc_gain = app.agc_gain + (g_tgt - app.agc_gain) * alpha_gain;

    for v in &mut vals { *v *= app.agc_gain; }

    for v in &mut vals {
        let x = *v;
        *v = soft_sat(x, 1.8);
    }

    let m = vals.len();
    let mut tgt = vec![0.0f32; n];
    for k in 0..n {
        let t  = k as f32 * (m - 1) as f32 / (n - 1) as f32;
        let i0 = t.floor() as usize;
        let i1 = (i0 + 1).min(m - 1);
        let a = vals[i0];
        let b = vals[i1];
        tgt[k] = a + (b - a) * (t - i0 as f32);
    }

    for k in 1..n-1 { tgt[k] = 0.2 * tgt[k-1] + 0.6 * tgt[k] + 0.2 * tgt[k+1]; }

    let tau_up = 0.040;
    let tau_dn = 0.120;
    let a_up = 1.0 - (-dt / tau_up).exp();
    let a_dn = 1.0 - (-dt / tau_dn).exp();
    for i in 0..n {
        let cur = app.vis_draw[i];
        let des = tgt[i];
        let a = if des > cur {
            a_up
        } else {
            a_dn
        };
        app.vis_draw[i] = cur + (des - cur) * a;
    }
    for k in 1..n-1 {
        app.vis_draw[k] = (app.vis_draw[k-1] + 2.0*app.vis_draw[k] + app.vis_draw[k+1]) * 0.25;
    }

    let painter = ui.painter_at(band);
    let h = band.height() * 1.02;
    let baseline = band.bottom() - 4.0;
    let step = band.width() / (n as f32 - 1.0);

    let mut line: Vec<egui::Pos2> = Vec::with_capacity(n);
    for i in 0..n {
        let x = band.left() + i as f32 * step;
        let y = (baseline - h * app.vis_draw[i]).clamp(band.top(), baseline);
        line.push(egui::pos2(x, y));
    }

    let glow = egui::Stroke::new(6.0, egui::Color32::from_rgba_unmultiplied(
        app.accent.r(), app.accent.g(), app.accent.b(), 42));
    painter.add(egui::Shape::line(line.clone(), glow));

    let contour = egui::Stroke::new(1.8, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 170));
    painter.add(egui::Shape::line(line, contour));
}
