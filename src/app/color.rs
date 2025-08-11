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
pub(crate) fn best_on(c: Color32) -> Color32 {
    if rel_luminance(c) > 0.5 {
        Color32::from_rgb(5,5,7)
    } else {
        Color32::WHITE
    }
}
