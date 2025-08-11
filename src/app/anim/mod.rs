mod flow;
mod color;
mod state;
mod paint;
mod time_rng;

pub use paint::paint_bg_gradient;
pub use state::{
    ThemeAnim,
    tick_theme_anim,
    begin_theme_anim
};
