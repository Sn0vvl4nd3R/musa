use std::env;
use anyhow::Result;

use eframe::{
    egui,
    Renderer
};

mod app;
mod util;
mod track;
mod cover;
mod theme;
mod player;
mod config;
mod duration;
mod ui {
    pub mod widgets;
}

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
            force_x11,
        }
    }
}

fn main() -> Result<()> {
    let cfg_launch = LaunchCfg::from_env_and_args();
    if cfg_launch.force_x11 {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    let app_cfg = config::Config::load_or_create()?;

    let native_opts = eframe::NativeOptions {
        renderer: cfg_launch.renderer,
        vsync: if cfg_launch.vsync {
            true
        } else {
            app_cfg.window.vsync
        },
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(app_cfg.window.start_w, app_cfg.window.start_h))
            .with_min_inner_size(egui::vec2(app_cfg.window.min_w, app_cfg.window.min_h))
            .with_decorations(true),
        ..Default::default()
    };

    let app = app::MusaApp::new(app_cfg)?;
    eframe::run_native("MUSA - Music Player", native_opts, Box::new(|_| Box::new(app)))
        .map_err(|e| anyhow::anyhow!("GUI error: {e}"))
}
