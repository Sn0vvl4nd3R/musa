use egui;
use std::time::Duration;
use crate::theme::apply_visuals;

use eframe::{
    App,
    Frame
};

use super::{
    nav,
    anim,
    cover,
    pages,
    scanner,
    MusaApp,
    controls,
};

impl App for MusaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.update_timing();
        self.handle_input(ctx);
        self.tick_audio_reactivity();

        anim::tick_theme_anim(self, ctx);
        apply_visuals(ctx, self.accent);
        anim::paint_bg_gradient(ctx, self);

        cover::poll_cover_result(self, ctx);
        scanner::poll_scan_result(self, ctx);

        self.layout_ui(ctx);

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

impl MusaApp {
    fn update_timing(&mut self) {
        let now = std::time::Instant::now();
        self.dt_sec = (now - self.last_frame)
            .as_secs_f32()
            .clamp(0.001, 0.05);
        self.last_frame = now;
        self.bg_time += self.dt_sec;
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.player.toggle_pause();
            }
            if i.key_pressed(egui::Key::ArrowLeft) {
                if let Err(e) = self.player.prev() {
                    self.status = format!("Prev error: {e}");
                }
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                if let Err(e) = self.player.next() {
                    self.status = format!("Next error: {e}");
                }
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                let maxv = self.cfg.ui.volume.max_value;
                self.player
                    .set_volume((self.player.volume + 0.05).min(maxv));
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                self.player.set_volume((self.player.volume - 0.05).max(0.0));
            }
        });
    }

    fn tick_audio_reactivity(&mut self) {
        let recent = self.player.vis_buffer().take_recent(4096);

        let target = if recent.len() >= 256 {
            let mut v: Vec<f32> = recent.iter().map(|x| x.abs()).collect();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let idx = ((v.len() as f32 * 0.90) as usize).min(v.len() - 1);
            (v[idx] * 2.0).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let a_fast = 1.0 - (-self.dt_sec / 0.15).exp();
        self.bg_music_drive += (target - self.bg_music_drive) * a_fast;

        let a_slow = 1.0 - (-self.dt_sec / 0.90).exp();
        self.bg_music_env += (target - self.bg_music_env) * a_slow;

        let onset = (self.bg_music_drive - self.bg_music_env * 1.06).max(0.0);
        self.bg_pulse = (self.bg_pulse + onset * 2.4).min(1.0);
        self.bg_pulse *= (-self.dt_sec / 0.45).exp();

        let base = [0.010, 0.008, 0.006, 0.007];
        let speed_mul = 0.7 + 0.9 * self.bg_music_env;

        let targets = [
            base[0] * speed_mul,
            base[1] * speed_mul,
            base[2] * speed_mul,
            base[3] * speed_mul,
        ];
        let a_v = 1.0 - (-self.dt_sec / 0.70).exp();
        self.bg_speed1 += (targets[0] - self.bg_speed1) * a_v;
        self.bg_speed2 += (targets[1] - self.bg_speed2) * a_v;
        self.bg_speed3 += (targets[2] - self.bg_speed3) * a_v;
        self.bg_speed4 += (targets[3] - self.bg_speed4) * a_v;

        self.bg_phase1 = (self.bg_phase1 + self.bg_speed1 * self.dt_sec) % 1.0;
        self.bg_phase2 = (self.bg_phase2 + self.bg_speed2 * self.dt_sec) % 1.0;
        self.bg_phase3 = (self.bg_phase3 + self.bg_speed3 * self.dt_sec) % 1.0;
        self.bg_phase4 = (self.bg_phase4 + self.bg_speed4 * self.dt_sec) % 1.0;
    }

    fn layout_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("musa_top")
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.add_space(4.0);
                nav::nav_tabs(self, ui);
            });

        egui::TopBottomPanel::bottom("musa_bottom")
            .frame(egui::Frame::none())
            .resizable(false)
            .exact_height(self.cfg.ui.bottom_bar_h)
            .show(ctx, |ui| controls::bottom_controls(self, ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| match self.view {
                super::types::UiView::Settings => pages::settings::ui_page_settings(self, ui),
                super::types::UiView::Browser => pages::browser::ui_page_browser(self, ui),
                super::types::UiView::Player => pages::player::ui_page_player(self, ui),
                super::types::UiView::Playlist => pages::playlist::ui_page_playlist(self, ui),
            });
    }
}
