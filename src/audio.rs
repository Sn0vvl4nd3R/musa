use std::{fs::File, io, path::Path, time::Duration};

use rodio::{
    Decoder, Player, Source,
    stream::{DeviceSinkBuilder, MixerDeviceSink},
};

use crate::Result;

struct Backend {
    player: Player,
    _device: MixerDeviceSink,
}

pub struct AudioEngine {
    backend: Option<Backend>,
    volume: u8,
    total: Option<Duration>,
}

impl AudioEngine {
    pub fn new(volume: u8) -> Self {
        Self {
            backend: None,
            volume,
            total: None,
        }
    }

    fn ensure_backend(&mut self) -> Result<&mut Backend> {
        if self.backend.is_none() {
            let device = DeviceSinkBuilder::open_default_sink().map_err(|error| {
                io::Error::other(format!("failed to open the default audio device: {error}"))
            })?;
            let player = Player::connect_new(device.mixer());
            player.set_volume(self.volume as f32 / 100.0);
            self.backend = Some(Backend {
                player,
                _device: device,
            });
        }

        Ok(self.backend.as_mut().expect("backend was initialized"))
    }

    pub fn play_file(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path).map_err(|error| {
            io::Error::other(format!("failed to open {}: {error}", path.display()))
        })?;
        let source = Decoder::try_from(file).map_err(|error| {
            io::Error::other(format!("failed to decode {}: {error}", path.display()))
        })?;
        let total = source.total_duration();

        let backend = self.ensure_backend()?;
        backend.player.stop();
        backend.player.append(source);
        backend.player.play();
        self.total = total;
        Ok(())
    }

    pub fn pause(&self) {
        if let Some(backend) = &self.backend {
            backend.player.pause();
        }
    }

    pub fn resume(&self) {
        if let Some(backend) = &self.backend {
            backend.player.play();
        }
    }

    pub fn stop(&mut self) {
        if let Some(backend) = self.backend.take() {
            backend.player.stop();
        }
        self.total = None;
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
        if let Some(backend) = &self.backend {
            backend.player.set_volume(volume as f32 / 100.0);
        }
    }

    pub fn position(&self) -> Duration {
        self.backend
            .as_ref()
            .map_or(Duration::ZERO, |backend| backend.player.get_pos())
    }

    pub fn total(&self) -> Option<Duration> {
        self.total
    }

    pub fn is_empty(&self) -> bool {
        self.backend
            .as_ref()
            .is_none_or(|backend| backend.player.empty())
    }

    pub fn seek_by(&self, seconds: i64) -> Result<()> {
        let current = self.position().as_secs_f64();
        self.seek_to((current + seconds as f64).max(0.0))
    }

    pub fn seek_to(&self, seconds: f64) -> Result<()> {
        let Some(backend) = &self.backend else {
            return Err(io::Error::other("audio backend is not active"));
        };
        let target = match self.total {
            Some(total) => seconds.clamp(0.0, total.as_secs_f64()),
            None => seconds.max(0.0),
        };

        backend
            .player
            .try_seek(Duration::from_secs_f64(target))
            .map_err(|error| io::Error::other(format!("seek failed: {error}")))?;
        Ok(())
    }
}
