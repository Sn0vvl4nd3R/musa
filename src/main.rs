use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    name = "Musa",
    about = "Yet another music player"
)]
struct Args {
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    #[arg(long, short = 'r', action = ArgAction::SetTrue)]
    recursive: bool,
}

fn is_audio_file(p: &Path) -> bool {
    match p.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
        Some(ext) => matches!(
            ext.as_str(),
            "mp3" | "flac" | "ogg" | "opus" | "wav" | "aiff" | "aif" | "alac" | "m4a"
        ),
        None => false,
    }
}

fn collect_playlist(inputs: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for input in inputs {
        if input.is_file() {
            if is_audio_file(input) {
                files.push(input.canonicalize()?);
            }
        } else if input.is_dir() {
            if recursive {
                for entry in WalkDir::new(input).into_iter().filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.is_file() && is_audio_file(p) {
                        files.push(p.canonicalize()?);
                    }
                }
            } else {
                for entry in input.read_dir().with_context(|| {
                    format!("Failed to read directory: {}", input.display())
                })? {
                    let e = entry?;
                    let p = e.path();
                    if p.is_file() && is_audio_file(&p) {
                        files.push(p.canonicalize()?);
                    }
                }
            }
        }
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(anyhow!("No audio files were found"));
    }
    Ok(files)
}

fn open_decoder(path: &Path) -> Result<Decoder<BufReader<File>>> {
    let f = File::open(path).with_context(|| format!("Can't open file: {}", path.display()))?;
    let reader = BufReader::new(f);
    Decoder::new(reader).map_err(|e| anyhow!("rodio::Decoder error for {}: {e}", path.display()))
}

struct Player {
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    volume: f32,
}

impl Player {
    fn new(handle: OutputStreamHandle) -> Self {
        Self {
            handle,
            sink: None,
            volume: 1.0,
        }
    }

    fn stop(&mut self) {
        if let Some(s) = &self.sink {
            s.stop();
        }
        self.sink = None;
    }

    fn play_file(&mut self, path: &Path) -> Result<()> {
        let dec = open_decoder(path)?;
        let sink = Sink::try_new(&self.handle).map_err(|e| anyhow!("Не создать аудио sink: {e}"))?;
        sink.set_volume(self.volume);
        sink.append(dec.convert_samples::<f32>());
        sink.play();
        self.sink = Some(sink);
        Ok(())
    }

    fn toggle_pause(&self) {
        if let Some(s) = &self.sink {
            if s.is_paused() {
                s.play();
            } else {
                s.pause();
            }
        }
    }

    fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 2.0);
        if let Some(s) = &self.sink {
            s.set_volume(self.volume);
        }
    }

    fn is_playing(&self) -> bool {
        self.sink.as_ref().map(|s| !s.is_paused()).unwrap_or(false)
    }

    fn is_sink_empty(&self) -> bool {
        self.sink.as_ref().map(|s| s.empty()).unwrap_or(true)
    }
}

fn print_help() {
    eprintln!(
        "\nHotkeys:
        p - pause / resume
        n - next track
        b - previous track
        + - louder
        - - quieter
        q - exit\n"
    );
}

#[derive(Debug)]
enum Cmd {
    PauseToggle,
    Next,
    Prev,
    VolUp,
    VolDown,
    Quit,
    Unknown(String),
}

fn parse_cmd(s: &str) -> Cmd {
    match s.trim() {
        "p" => Cmd::PauseToggle,
        "n" => Cmd::Next,
        "b" => Cmd::Prev,
        "+" => Cmd::VolUp,
        "-" => Cmd::VolDown,
        "q" => Cmd::Quit,
        other if other.is_empty() => Cmd::Unknown(String::new()),
        other => Cmd::Unknown(other.to_string()),
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let playlist = collect_playlist(&args.inputs, args.recursive)?;
    eprintln!("Found tracks: {}", playlist.len());
    print_help();

    let (_stream, handle) =
        OutputStream::try_default().map_err(|e| anyhow!("Audio output device not found: {e}"))?;

    let mut player = Player::new(handle);
    let mut idx: usize = 0;

    eprintln!("[Playing]  {}", playlist[idx].display());
    player.play_file(&playlist[idx])?;

    let (tx, rx) = mpsc::channel::<Cmd>();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let s = match line {
                Ok(v) => v,
                Err(_) => break,
            };
            let _ = tx.send(parse_cmd(&s));
        }
    });

    'mainloop: loop {
        if let Ok(cmd) = rx.recv_timeout(Duration::from_millis(150)) {
            match cmd {
                Cmd::PauseToggle => {
                    player.toggle_pause();
                    eprintln!("{}", if player.is_playing() { "[Playing]  play" } else { "[Paused]" });
                }
                Cmd::Next => {
                    player.stop();
                    idx = (idx + 1) % playlist.len();
                    eprintln!("[Playing]  {}", playlist[idx].display());
                    if let Err(e) = player.play_file(&playlist[idx]) {
                        eprintln!("Playback error: {e}");
                    }
                }
                Cmd::Prev => {
                    player.stop();
                    idx = if idx == 0 { playlist.len() - 1 } else { idx - 1 };
                    eprintln!("[Playing]  {}", playlist[idx].display());
                    if let Err(e) = player.play_file(&playlist[idx]) {
                        eprintln!("Playback error: {e}");
                    }
                }
                Cmd::VolUp => {
                    let new_v = (player.volume + 0.1).clamp(0.0, 2.0);
                    player.set_volume(new_v);
                    eprintln!("[+] volume: {:.1}", player.volume);
                }
                Cmd::VolDown => {
                    let new_v = (player.volume - 0.1).clamp(0.0, 2.0);
                    player.set_volume(new_v);
                    eprintln!("[-] volume: {:.1}", player.volume);
                }
                Cmd::Quit => {
                    eprintln!("Quit");
                    break 'mainloop;
                }
                Cmd::Unknown(s) => {
                    if !s.is_empty() {
                        eprintln!("Unknown command: {}", s);
                        print_help();
                    }
                }
            }
        }

        if player.is_sink_empty() {
            idx = (idx + 1) % playlist.len();
            eprintln!("[Playing]  {}", playlist[idx].display());
            if let Err(e) = player.play_file(&playlist[idx]) {
                eprintln!("Playback error: {e}");
            }
        }
    }

    Ok(())
}

