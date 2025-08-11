pub mod ui;
pub mod color;
pub mod palette;

pub use color::lerp_srgb;
pub use ui::apply_visuals;
pub use palette::{
    extract_palette,
    title_from_accent,
    make_gradient_stops,
    accent_from_palette
};
