use rodio::Source;
use anyhow::Result;
use crate::duration::probed_duration;

use std::time::{
    Instant,
    Duration,
};

use super::{
    Player,
    io::open_decoder
};

impl Player {
    pub fn play_current_from(&mut self, pos: Duration) -> Result<()> {
        let Some(p) = self.current_path() else {
            return Ok(());
        };

        let mut total = probed_duration(p);
        let dec = open_decoder(p)?;
        if total.is_none() {
            total = dec.total_duration();
        }
        self.track_total = total;

        self.stop();
        self.start_sink_with_source(dec, pos)
    }

    pub fn play_current(&mut self) -> Result<()> {
        self.play_current_from(Duration::ZERO)
    }

    pub fn toggle_pause(&mut self) {
        if let Some(s) = &self.sink {
            if s.is_paused() {
                s.play();
                self.pos_started_at = Some(Instant::now());
            } else {
                let now_pos = if let Some(st) = self.pos_started_at {
                    self.pos_base.saturating_add(st.elapsed())
                } else {
                    self.pos_base
                };
                self.pos_base = now_pos;
                self.pos_started_at = None;
                s.pause();
            }
        }
    }

    pub fn seek_to_secs(&mut self, secs: f32) -> Result<()> {
        if !secs.is_finite() || secs < 0.0 {
            return Ok(());
        }
        self.play_current_from(Duration::from_secs_f32(secs))
    }

    pub fn next(&mut self) -> Result<()> {
        if self.playlist.is_empty() {
            return Ok(());
        }
        self.index = (self.index + 1) % self.playlist.len();
        self.play_current()
    }

    pub fn prev(&mut self) -> Result<()> {
        if self.playlist.is_empty() {
            return Ok(());
        }
        self.index = if self.index == 0 {
            self.playlist.len() - 1
        } else {
            self.index - 1
        };
        self.play_current()
    }

    pub fn auto_advance_if_needed(&mut self) -> Result<()> {
        if let Some(s) = &self.sink {
            if s.empty() && !self.playlist.is_empty() {
                if self.repeat_one {
                    return self.play_current();
                } else {
                    self.index = (self.index + 1) % self.playlist.len();
                    return self.play_current();
                }
            }
        }
        Ok(())
    }
}
