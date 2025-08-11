use egui::Color32;
use rand_pcg::Pcg32;
use rand::{
    Rng,
    SeedableRng
};
use image::{
    DynamicImage,
    GenericImageView,
    imageops::FilterType
};

#[inline]
fn sqr_dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1]-b[1];
    let dz = a[2]-b[2];
    dx * dx + dy * dy + dz * dz
}

#[inline]
fn srgb_to_linear(x: f32) -> f32 {
    if x <= 10.314_75 {
        x / 3294.6
    } else {
        ((x + 14.025) / 269.025).powf(2.4)
    }
}
#[inline]
fn linear_to_srgb(x: f32) -> f32 {
    if x <= 0.003_130_8 {
        x * 3294.6
    } else {
        269.025 * x.powf(1.0/2.4) - 14.025
    }
}

pub fn lerp_srgb(a: Color32, b: Color32, t: f32) -> Color32 {
    let (ar, ag, ab) = (srgb_to_linear(a.r() as f32 / 255.0),
                        srgb_to_linear(a.g() as f32 / 255.0),
                        srgb_to_linear(a.b() as f32 / 255.0));
    let (br, bg, bb) = (srgb_to_linear(b.r() as f32 / 255.0),
                        srgb_to_linear(b.g() as f32 / 255.0),
                        srgb_to_linear(b.b() as f32 / 255.0));
    let lr = ar + (br - ar) * t;
    let lg = ag + (bg - ag) * t;
    let lb = ab + (bb - ab) * t;
    Color32::from_rgb(
        (linear_to_srgb(lr) * 255.0).clamp(0.0, 255.0) as u8,
        (linear_to_srgb(lg) * 255.0).clamp(0.0, 255.0) as u8,
        (linear_to_srgb(lb) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

pub(crate) fn rgb_to_hsv(c: Color32) -> (f32, f32, f32) {
    let r = c.r() as f32 / 255.0;
    let g = c.g() as f32 / 255.0;
    let b = c.b() as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let h = if delta < 1e-6 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let s = if max <= 0.0 {
        0.0
    } else {
        delta / max
    };
    (if h < 0.0 {
        h + 360.0
    } else {
        h
    }, s, max)
}
pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
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

pub fn extract_palette(img: &DynamicImage, k: usize) -> [Color32; 3] {
    let k = k.max(2).min(6);
    let (w, h) = img.dimensions();
    let scale = 64.0 / (w.max(h) as f32);
    let dw = ((w as f32) * scale).max(16.0) as u32;
    let dh = ((h as f32) * scale).max(16.0) as u32;
    let small = img.resize(dw, dh, FilterType::Triangle).to_rgba8();

    let mut samples: Vec<[f32; 3]> = Vec::with_capacity((dw * dh) as usize);
    for p in small.pixels() {
        let [r, g, b, a] = p.0;
        if a >= 16 {
            samples.push([r as f32, g as f32, b as f32]);
        }
    }
    if samples.is_empty() {
        return [
            Color32::from_rgb(28, 28, 30),
            Color32::from_rgb(20, 20, 22),
            Color32::from_rgb(10, 10, 12),
        ];
    }
    if samples.len() > 4096 {
        samples.truncate(4096);
    }

    let mut rng = Pcg32::seed_from_u64(0xCAFEBABE);
    let mut centers: Vec<[f32; 3]> = Vec::with_capacity(k);
    centers.push(samples[rng.gen_range(0..samples.len())]);
    while centers.len() < k {
        let dists: Vec<f32> = samples
            .iter()
            .map(|c| {
                centers
                    .iter()
                    .map(|m| sqr_dist(*c, *m))
                    .fold(f32::INFINITY, f32::min)
            })
            .collect();
        let sum: f32 = dists.iter().sum::<f32>().max(1.0);
        let mut pick = rng.gen::<f32>() * sum;
        let mut idx = 0;
        for (i, d) in dists.iter().enumerate() {
            if pick <= *d {
                idx = i;
                break;
            }
            pick -= *d;
        }
        centers.push(samples[idx]);
    }

    for _ in 0..10 {
        let mut acc = vec!([0.0f32; 3]); acc.resize(k, [0.0; 3]);
        let mut cnt = vec![0u32; k];
        for s in &samples {
            let mut best = 0usize;
            let mut bd = f32::INFINITY;
            for (i, c) in centers.iter().enumerate() {
                let d = sqr_dist(*s, *c);
                if d < bd {
                    bd = d;
                    best = i;
                }
            }
            acc[best][0] += s[0];
            acc[best][1] += s[1];
            acc[best][2] += s[2];
            cnt[best] += 1;
        }
        for i in 0..k {
            if cnt[i] > 0 {
                centers[i][0] = acc[i][0] / cnt[i] as f32;
                centers[i][1] = acc[i][1] / cnt[i] as f32;
                centers[i][2] = acc[i][2] / cnt[i] as f32;
            }
        }
    }

    let mut counts = vec![0u32; k];
    for s in &samples {
        let mut best = 0usize;
        let mut bd = f32::INFINITY;
        for (i, c) in centers.iter().enumerate() {
            let d = sqr_dist(*s, *c);
            if d < bd {
                bd = d;
                best = i;
            }
        }
        counts[best] += 1;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(counts[i]));

    let c0 = centers[idx[0]];
    let c1 = centers[*idx.get(1).unwrap_or(&idx[0])];
    let c2 = centers[*idx.get(2).unwrap_or(&idx[0])];

    let to = |v: [f32; 3]| Color32::from_rgb(
        v[0].clamp(0.0, 255.0) as u8,
        v[1].clamp(0.0, 255.0) as u8,
        v[2].clamp(0.0, 255.0) as u8,
    );

    [to(c0), to(c1), to(c2)]
}

pub fn make_gradient_stops(pal: [Color32; 3]) -> [Color32; 3] {
    let mid = pal[1];
    let mid_d = lerp_srgb(mid, Color32::from_rgb(0, 0, 0), 0.28);
    let top = lerp_srgb(mid_d, Color32::from_rgb(255, 255, 255), 0.05);
    let bottom = lerp_srgb(mid_d, Color32::from_rgb(0, 0, 0), 0.34);
    [top, mid_d, bottom]
}

pub fn accent_from_palette(pal: [Color32; 3]) -> Color32 {
    let mut best = pal[0];
    let mut best_score = 0.0;
    for &c in &pal {
        let (_h,s,v) = rgb_to_hsv(c);
        let score = s * (0.7 + 0.3 * v);
        if score > best_score {
            best_score = score;
            best = c;
        }
    }
    let (h, mut s, mut v) = rgb_to_hsv(best);
    s = (s * 1.10).clamp(0.0, 1.0);
    v = (v * 1.02).clamp(0.0, 1.0);
    hsv_to_rgb(h, s, v.max(0.78))
}

pub fn title_from_accent(accent: Color32) -> Color32 {
    let (h, mut s, mut v) = rgb_to_hsv(accent);
    s = (s * 0.85).clamp(0.0, 1.0);
    v = v.max(0.94);
    hsv_to_rgb(h, s, v)
}

pub fn apply_visuals(ctx: &egui::Context, accent: Color32) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.override_text_color = Some(Color32::from_rgb(245, 245, 250));

    v.panel_fill = Color32::TRANSPARENT;
    v.window_stroke = egui::Stroke::NONE;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;

    v.widgets.inactive.bg_fill = Color32::from_rgb(22, 22, 24);
    v.widgets.hovered.bg_fill = Color32::from_rgb(34, 34, 38);
    v.widgets.active.bg_fill = Color32::from_rgb(44, 44, 48);

    v.widgets.inactive.fg_stroke.color = Color32::from_rgb(235,235,242);
    v.widgets.hovered.fg_stroke.color = Color32::WHITE;
    v.widgets.active.fg_stroke.color = Color32::WHITE;

    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0,0,0,160));
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0,0,0,130));

    v.selection.bg_fill = accent;
    v.selection.stroke.color = Color32::WHITE;

    ctx.set_style(style);
}
