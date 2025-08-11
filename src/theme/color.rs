use egui::Color32;

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

pub fn rgb_to_hsv(c: Color32) -> (f32, f32, f32) {
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
    let h = if h < 0.0 {
        h + 360.0
    } else {
        h
    };
    (h, s, max)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
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
