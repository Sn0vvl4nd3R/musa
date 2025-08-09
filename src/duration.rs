use std::{
    fs::File,
    path::Path,
    time::Duration
};
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions
};
use symphonia::default::get_probe;

pub fn probed_duration(path: &Path) -> Option<Duration> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = get_probe()
        .format(&Default::default(), mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let mut format = probed.format;
    let track = format.default_track()?;
    let params = &track.codec_params;

    if let (Some(tb), Some(n_frames)) = (params.time_base, params.n_frames) {
        let t = tb.calc_time(n_frames);
        let secs = t.seconds as f64 + t.frac;
        return Some(Duration::from_secs_f64(secs));
    }
    if let Some(tb) = params.time_base {
        let mut last_ts: u64 = 0;
        while let Ok(pkt) = format.next_packet() {
            last_ts = pkt.ts;
        }
        let t = tb.calc_time(last_ts);
        let secs = t.seconds as f64 + t.frac;
        return Some(Duration::from_secs_f64(secs));
    }
    None
}
