use egui::Color32;

#[inline]
pub(super) fn c32(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

#[inline]
pub(super) fn to_rgb(c: Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}
