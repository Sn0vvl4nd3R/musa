use std::{fs::File, io, path::Path, time::Duration};

use rodio::{
    Decoder, Player, Source,
    stream::{DeviceSinkBuilder, MixerDeviceSink},
};

use crate::Result;

pub struct AudioEngine {
    _device: MixerDeviceSink,
    player: Player,
    total: Option<Duration>,
}

impl AudioEngine {
    pub fn new(volume: u8) -> Result<Self> {
        let device = DeviceSinkBuilder::open_default_sink().map_err(|error| {
            io::Error::other(format!("failed to open the default audio device: {error}"))
        })?;
        let player = Player::connect_new(device.mixer());
        player.set_volume(volume as f32 / 100.0);

        Ok(Self {
            _device: device,
            player,
            total: None,
        })
    }

    pub fn play_file(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path).map_err(|error| {
            io::Error::other(format!("failed to open {}: {error}", path.display()))
        })?;
        let source = Decoder::try_from(file).map_err(|error| {
            io::Error::other(format!("failed to decode {}: {error}", path.display()))
        })?;

        self.total = source.total_duration();
        self.player.stop();
        self.player.append(source);
        self.player.play();
        Ok(())
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn resume(&self) {
        self.player.play();
    }

    pub fn stop(&mut self) {
        self.player.stop();
        self.total = None;
    }

    pub fn set_volume(&self, volume: u8) {
        self.player.set_volume(volume as f32 / 100.0);
    }

    pub fn position(&self) -> Duration {
        self.player.get_pos()
    }

    pub fn total(&self) -> Option<Duration> {
        self.total
    }

    pub fn is_empty(&self) -> bool {
        self.player.empty()
    }

    pub fn seek_by(&self, seconds: i64) -> Result<()> {
        let current = self.position().as_secs_f64();
        self.seek_to((current + seconds as f64).max(0.0))
    }

    pub fn seek_to(&self, seconds: f64) -> Result<()> {
        let target = match self.total {
            Some(total) => seconds.clamp(0.0, total.as_secs_f64()),
            None => seconds.max(0.0),
        };

        self.player
            .try_seek(Duration::from_secs_f64(target))
            .map_err(|error| io::Error::other(format!("seek failed: {error}")))?;
        Ok(())
    }
}
