use regex::Regex;
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

pub fn seconds_to_mmss(secs: f32) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".to_string();
    }
    let s = secs as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub fn strip_year_prefix(mut s: String) -> String {
    let re1 = Regex::new(r"(?i)^\s*\d{4}\s*[-_.]\s*").unwrap();
    let re2 = Regex::new(r"(?i)^\s*[\(\[]\s*\d{4}\s*[\)\]]\s*[-_.]?\s*").unwrap();
    s = re1.replace(&s, "").to_string();
    s = re2.replace(&s, "").to_string();
    s.trim().to_string()
}
pub fn strip_year_suffix(mut s: String) -> String {
    let re = Regex::new(r"(?i)\s*[\(\[]\s*\d{4}\s*[\)\]]\s*$").unwrap();
    s = re.replace(&s, "").to_string();
    s.trim().to_string()
}
pub fn strip_tech_brackets(mut s: String) -> String {
    let re = Regex::new(
        r#"(?ix)\s*[\(\[][^)\]]*(?:kHz|Hz|bit|kbps|VBR|CBR|FLAC|ALAC|MP3|AAC|OGG|OPUS|DSD|mono|stereo)[^)\]]*[\)\]]"#,
    ).unwrap();
    loop {
        let t = re.replace(&s, "").to_string();
        if t == s {
            break;
        }
        s = t;
    }
    s.trim().to_string()
}

pub fn strip_artist_prefix(mut album: String, artist: &str) -> String {
    for sep in [" - ", " – ", " — "] {
        let pref = format!("{}{}", artist, sep);
        if album.to_ascii_lowercase().starts_with(&pref.to_ascii_lowercase()) {
            album = album[pref.len()..].to_string();
            break;
        }
    }
    album
}

pub fn normalize_ws(mut s: String) -> String {
    s = s.replace('_', " ");
    let re_ws = Regex::new(r"\s+").unwrap();
    s = re_ws.replace_all(&s, " ").to_string();
    s.trim().to_string()
}

pub fn clean_album_for_display(raw_album: &str, artist: &str) -> String {
    let mut s = raw_album.trim().to_string();
    s = strip_year_prefix(s);
    s = strip_year_suffix(s);
    s = strip_tech_brackets(s);
    s = strip_artist_prefix(s, artist);
    normalize_ws(s)
}
