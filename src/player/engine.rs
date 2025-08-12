use crate::track::Track;
use super::vis::VisBuffer;

use std::{
    path::Path,
    sync::Arc,
    time::{
        Instant,
        Duration,
    },
};

use rodio::{
    Sink,
    OutputStream,
    OutputStreamHandle,
};

pub struct Player {
    pub(crate) stream: Option<OutputStream>,
    pub(crate) handle: Option<OutputStreamHandle>,
    pub(crate) sink: Option<Sink>,

    pub playlist: Vec<Track>,
    pub index: usize,
    pub volume: f32,
    pub track_total: Option<Duration>,
    pub repeat_one: bool,

    pub(crate) pos_base: Duration,
    pub(crate) pos_started_at: Option<Instant>,

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
            repeat_one: false,
            pos_base: Duration::ZERO,
            pos_started_at: None,
            vis_buf: VisBuffer::new(96_000),
        }
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
    pub fn vis_buffer(&self) -> Arc<VisBuffer> {
        self.vis_buf.clone()
    }
    pub fn toggle_repeat_one(&mut self) {
        self.repeat_one = !self.repeat_one;
    }
}
