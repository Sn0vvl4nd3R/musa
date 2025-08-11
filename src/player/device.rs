use anyhow::{
    anyhow,
    Result,
};

use super::{
    Player,
    tap::TapSource
};

use std::{
    fs::File,
    io::BufReader,
    time::Instant
};

use rodio::{
    Sink,
    Source,
    Decoder,
    OutputStream,
};

impl Player {
    pub(crate) fn ensure_stream(&mut self) -> Result<()> {
        if self.handle.is_none() {
            let (stream, handle) =
                OutputStream::try_default().map_err(|e| anyhow!("No audio device: {e}"))?;
            self.stream = Some(stream);
            self.handle = Some(handle);
        }
        Ok(())
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

    pub(crate) fn start_sink_with_source(
        &mut self,
        dec: Decoder<BufReader<File>>,
        skip: std::time::Duration,
    ) -> Result<()> {
        self.ensure_stream()?;
        let src = dec.skip_duration(skip).convert_samples::<f32>();
        let tap = TapSource::new(src, self.vis_buffer());
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
}
