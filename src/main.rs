use eframe::{
    egui,
    Renderer
};
use anyhow::Result;
use std::env;
mod app;
mod player;
mod track;
mod util;
mod duration;
mod cover;
mod ui {
    pub mod widgets;
}
mod theme;

#[derive(Clone, Copy)]
struct LaunchCfg {
    renderer: Renderer,
    vsync: bool,
    force_x11: bool,
}

impl LaunchCfg {
    fn from_env_and_args() -> Self {
        let mut renderer = Renderer::Glow;
        let mut vsync = false;
        let mut force_x11 = false;

        if let Ok(s) = env::var("MUSA_RENDERER") {
            match s.to_ascii_lowercase().as_str() {
                "glow" => renderer = Renderer::Glow,
                "wgpu" => {
                    #[cfg(feature = "wgpu")] {
                        renderer = Renderer::Wgpu;
                    }
                    #[cfg(not(feature = "wgpu"))] {
                        eprintln!("MUSA_RENDERER=wgpu set, but eframe without 'wgpu'. Using Glow.");
                    }
                }
                _ => {}
            }
        }
        if let Ok(s) = env::var("MUSA_VSYNC") {
            vsync = matches!(s.as_str(), "1" | "true" | "TRUE");
        }
        if env::var_os("MUSA_FORCE_X11").is_some() {
            force_x11 = true;
        }
        
        for arg in env::args().skip(1) {
            match arg.as_str() {
                "--glow" => renderer = Renderer::Glow,
                "--wgpu" => {
                    #[cfg(feature = "wgpu")] {
                        renderer = Renderer::Wgpu;
                    }
                    #[cfg(not(feature = "wgpu"))] {
                        eprintln!("--wgpu requested, but no 'wgpu' feature. Using Glow.");
                    }
                }
                "--vsync" => vsync = true,
                "--no-vsync" => vsync = false,
                "--x11" => force_x11 = true,
                _ => {}
            }
        }

        Self {
            renderer,
            vsync,
            force_x11
        }
    }
}

fn main() -> Result<()> {
    let cfg = LaunchCfg::from_env_and_args();
    if cfg.force_x11 {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    let min_w = 780.0;
    let min_h = 660.0;

    let native_opts = eframe::NativeOptions {
        renderer: cfg.renderer,
        vsync: cfg.vsync,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(900.0, 700.0))
            .with_min_inner_size(egui::vec2(min_w, min_h))
            .with_decorations(true),
        ..Default::default()
    };

    let app = app::MusaApp::new()?;
    eframe::run_native("MUSA - Music Player", native_opts, Box::new(|_| Box::new(app)))
        .map_err(|e| anyhow::anyhow!("GUI error: {e}"))
}
