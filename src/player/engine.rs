use anyhow::{
    anyhow,
    Result,
};
use rodio::{
    Sink,
    Source,
    Decoder,
    OutputStream,
    OutputStreamHandle,
};
use std::{
    fs::File,
    sync::Arc,
    path::Path,
    io::BufReader,
    time::{
        Instant,
        Duration,
    },
};

use crate::{
    track::Track,
    duration::probed_duration,
};
use super::{
    vis::VisBuffer,
    tap::TapSource,
    io::open_decoder,
};

pub struct Player {
    stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    sink:   Option<Sink>,
    pub playlist: Vec<Track>,
    pub index: usize,
    pub volume: f32,
    pub track_total: Option<Duration>,
    pos_base: Duration,
    pos_started_at: Option<Instant>,
    vis_buf: Arc<VisBuffer>,
}

impl Player {
    pub fn new() -> Self {
        Self {
            stream: None,
            handle: None,
            sink: None,
            playlist: Vec::new(),
            index: 0,
            volume: 1.0,
            track_total: None,
            pos_base: Duration::ZERO,
            pos_started_at: None,
            vis_buf: VisBuffer::new(96_000),
        }
    }

    fn ensure_stream(&mut self) -> Result<()> {
        if self.handle.is_none() {
            let (stream, handle) =
                OutputStream::try_default().map_err(|e| anyhow!("No audio device: {e}"))?;
            self.stream = Some(stream);
            self.handle = Some(handle);
        }
        Ok(())
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.playlist.get(self.index)
    }
    pub fn current_path(&self) -> Option<&Path> {
        self.current_track().map(|t| t.path.as_path())
    }
    pub fn current_album_dir(&self) -> Option<std::path::PathBuf> {
        self.current_track().map(|t| t.album_dir.clone())
    }
    pub fn is_playing(&self) -> bool {
        self.sink.as_ref().map(|s| !s.is_paused()).unwrap_or(false)
    }
    pub fn current_total_secs(&self) -> f32 {
        self.track_total.map(|d| d.as_secs_f32()).unwrap_or(f32::NAN)
    }

    pub fn current_pos(&self) -> Duration {
        if let Some(st) = self.pos_started_at {
            if self.is_playing() {
                return self.pos_base.saturating_add(st.elapsed());
            }
        }
        self.pos_base
    }

    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 2.0);
        if let Some(s) = &self.sink {
            s.set_volume(self.volume);
        }
    }

    pub fn stop(&mut self) {
        if let Some(s) = &self.sink {
            s.stop();
        }
        self.sink = None;
        self.pos_started_at = None;
    }

    fn start_sink_with_source(
        &mut self,
        dec: Decoder<BufReader<File>>,
        skip: Duration,
    ) -> Result<()> {
        self.ensure_stream()?;
        let src = dec.skip_duration(skip).convert_samples::<f32>();
        let tap = TapSource::new(src, self.vis_buf.clone());
        let handle = self.handle.as_ref().unwrap();
        let sink = Sink::try_new(handle).map_err(|e| anyhow!("Failed to create sink: {e}"))?;
        sink.set_volume(self.volume);
        sink.append(tap);
        sink.play();
        self.sink = Some(sink);
        self.pos_base = skip;
        self.pos_started_at = Some(Instant::now());
        Ok(())
    }

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
                self.index = (self.index + 1) % self.playlist.len();
                return self.play_current();
            }
        }
        Ok(())
    }

    pub fn vis_buffer(&self) -> Arc<VisBuffer> {
        self.vis_buf.clone()
    }
}
