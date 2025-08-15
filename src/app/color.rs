use egui::Color32;

#[inline]
fn srgb_channel_to_linear(u: f32) -> f32 {
    if u <= 0.04045 {
        u / 12.92
    } else {
        ((u + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn rel_luminance(c: Color32) -> f32 {
    let r = srgb_channel_to_linear(c.r() as f32 / 255.0);
    let g = srgb_channel_to_linear(c.g() as f32 / 255.0);
    let b = srgb_channel_to_linear(c.b() as f32 / 255.0);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

#[inline]
fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (l1, l2) = (rel_luminance(a), rel_luminance(b));
    let (hi, lo) = if l1 > l2 {
        (l1, l2)
    } else {
        (l2, l1)
    };
    (hi + 0.05) / (lo + 0.05)
}

#[inline]
pub(crate) fn best_on(bg: Color32) -> Color32 {
    let light = Color32::from_rgb(250, 250, 252);
    let dark = Color32::from_rgb(10, 10, 12);

    let cl = contrast_ratio(light, bg);
    let cd = contrast_ratio(dark, bg);

    if cl >= 4.5 || cd >= 4.5 {
        if cl >= cd {
            light
        } else {
            dark
        }
    } else {
        if cl >= cd {
            light
        } else {
            dark
        }
    }
}
