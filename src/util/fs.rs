use std::path::{
    Path,
    PathBuf
};

pub fn is_audio_file(p: &Path) -> bool {
    match p.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
        Some(ext) => matches!(
            ext.as_str(),
            "mp3" | "flac" | "ogg" | "opus" | "wav" | "aiff" | "aif" | "alac"
                | "m4a" | "m4b" | "mp4" | "aac" | "webm"
        ),
        None => false,
    }
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}
