use regex::Regex;
use std::path::{
    Component,
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

pub fn normalize_ws(mut s: String) -> String {
    s = s.replace('_', " ");
    let re_ws = Regex::new(r"\s+").unwrap();
    s = re_ws.replace_all(&s, " ").to_string();
    s.trim().to_string()
}
fn strip_year_prefix(mut s: String) -> String {
    let re1 = Regex::new(r"(?i)^\s*\d{4}\s*[-_.]\s*").unwrap();
    let re2 = Regex::new(r"(?i)^\s*[\(\[]\s*\d{4}\s*[\)\]]\s*[-_.]?\s*").unwrap();
    s = re1.replace(&s, "").to_string();
    s = re2.replace(&s, "").to_string();
    s.trim().to_string()
}
fn strip_year_suffix(mut s: String) -> String {
    let re = Regex::new(r"(?i)\s*[\(\[]\s*\d{4}\s*[\)\]]\s*$").unwrap();
    s = re.replace(&s, "").to_string();
    s.trim().to_string()
}
fn strip_tech_brackets(mut s: String) -> String {
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

fn strip_catalog_suffix(mut s: String) -> String {
    let re = Regex::new(r#"(?xi)\s*\[(?:[A-Z]{1,5}\s*)?\d[\d\s\-]*\]\s*$"#).unwrap();
    loop {
        let t = re.replace(&s, "").to_string();
        if t == s {
            break;
        }
        s = t;
    }
    s.trim().to_string()
}

fn strip_artist_prefix(mut album: String, artist: &str) -> String {
    for sep in [" - ", " – ", " — "] {
        let pref = format!("{}{}", artist, sep);
        if album.to_ascii_lowercase().starts_with(&pref.to_ascii_lowercase()) {
            album = album[pref.len()..].to_string();
            break;
        }
    }
    album
}

pub fn clean_album_for_display(raw_album: &str, artist: &str) -> String {
    let mut s = raw_album.trim().to_string();
    s = strip_year_prefix(s);
    s = strip_year_suffix(s);
    s = strip_tech_brackets(s);
    s = strip_catalog_suffix(s);
    s = strip_artist_prefix(s, artist);
    normalize_ws(s)
}

const BUCKETS: &[&str] = &[
    "albums", "album", "singles", "single", "eps", "ep", "compilations", "compilation",
    "records", "releases", "collections", "library", "audio", "music", "media",
    "disc", "discs", "disk", "cd", "cds", "cd1", "cd2", "cd3", "dvd",
    "tracks", "track", "songs", "song",
];

fn is_bucket(name: &str) -> bool {
    BUCKETS.iter().any(|b| name.eq_ignore_ascii_case(b))
}

fn parse_disc_label(s: &str) -> Option<u32> {
    let re = Regex::new(r"(?i)\b(?:cd|disc|disk)\s*-?\s*(\d{1,2})\b").unwrap();
    re.captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

fn parse_track_number(s: &str) -> (Option<u32>, Option<u32>, String) {
    let re_disc = Regex::new(
        r#"(?x) ^ \s*
           (?P<disc>\d{1,2}) \s* [-_\.] \s*
           (?P<track>\d{1,3})
           \s* [\.\)\-_]+ \s*
           (?P<rest>.+)
        "#
    ).unwrap();
    if let Some(c) = re_disc.captures(s) {
        let disc = c["disc"].parse::<u32>().ok();
        let track = c["track"].parse::<u32>().ok();
        let rest = c["rest"].trim().to_string();
        return (track, disc, rest);
    }
    let re = Regex::new(r#"(?x) ^ \s* (?P<track>\d{1,3}) \s* [\.\)\-_]+ \s* (?P<rest>.+) "#).unwrap();
    if let Some(c) = re.captures(s) {
        let track = c["track"].parse::<u32>().ok();
        let rest = c["rest"].trim().to_string();
        return (track, None, rest);
    }
    let re_only = Regex::new(r#"^\s*(?P<track>\d{1,3})\s*$"#).unwrap();
    if let Some(c) = re_only.captures(s) {
        let track = c["track"].parse::<u32>().ok();
        return (track, None, "".to_string());
    }
    (None, None, s.trim().to_string())
}

fn maybe_split_artist_title(file_stem: &str, known_artist: Option<&str>) -> Option<(String, String)> {
    if let Some(idx) = file_stem.find(" - ") {
        if let Some(ka) = known_artist {
            let (left, right) = file_stem.split_at(idx);
            let right = &right[3..];
            if left.trim().eq_ignore_ascii_case(ka.trim()) {
                return Some((left.trim().to_string(), right.trim().to_string()));
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct TrackMeta {
    pub artist: String,
    pub album: String,
    pub title: String,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
}

#[derive(Debug, Default)]
struct TagFields {
    artist: Option<String>,
    album_artist: Option<String>,
    album: Option<String>,
    title: Option<String>,
    track_no: Option<u32>,
    disc_no: Option<u32>,
    year: Option<u32>,
    genre: Option<String>,
}

fn read_tags(path: &Path) -> Option<TagFields> {
    use audiotags::Tag;

    let tag = Tag::new().read_from_path(path).ok()?;

    let mut tf = TagFields::default();
    tf.title = tag.title().map(str::to_string);
    tf.album = tag.album_title().map(str::to_string);
    tf.artist = tag.artist().map(str::to_string);
    tf.album_artist = tag.album_artist().map(str::to_string);
    tf.genre = tag.genre().map(str::to_string);
    tf.track_no = tag.track_number().map(|n| n as u32);
    tf.disc_no = tag.disc_number().map(|n| n as u32);
    tf.year = tag.year().and_then(|y| if y >= 0 {
        Some(y as u32)
    } else {
        None
    });

    Some(tf)
}

fn infer_artist_album_from_path(p: &Path) -> (Option<String>, Option<String>, Option<u32>) {
    let mut parts: Vec<String> = Vec::new();
    for c in p.components() {
        if let Component::Normal(os) = c {
            parts.push(os.to_string_lossy().to_string());
        }
    }

    let mut cand: Vec<(usize, String)> = Vec::new();
    for (i, name) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let stem = if last {
            Path::new(name)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| name.clone())
        } else {
            name.clone()
        };
        if !is_bucket(&stem) {
            cand.push((i, stem));
        }
    }

    let mut disc_no: Option<u32> = None;
    for (_, seg) in cand.iter().rev().take(3) {
        if let Some(d) = parse_disc_label(seg) {
            disc_no = Some(d);
            break;
        }
    }

    if cand.len() >= 3 {
        let album_raw = &cand[cand.len() - 2].1;
        let artist    = &cand[cand.len() - 3].1;
        return (Some(artist.clone()), Some(album_raw.clone()), disc_no);
    }

    (None, None, disc_no)
}

pub fn parse_track_meta(path: &Path) -> TrackMeta {
    let tf = read_tags(path).unwrap_or_default();

    let (artist_from_path, album_from_path_raw, disc_from_path) = infer_artist_album_from_path(path);

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let (track_from_name, disc_from_name, name_rest) = parse_track_number(stem);

    let artist = tf.artist
        .clone()
        .or_else(|| tf.album_artist.clone())
        .or(artist_from_path.clone())
        .unwrap_or_default();

    let album_raw = tf.album
        .clone()
        .or(album_from_path_raw.clone())
        .unwrap_or_default();

    let album = if artist.is_empty() { normalize_ws(album_raw) }
                else { clean_album_for_display(&album_raw, &artist) };

    let title0 = tf.title.clone().unwrap_or_else(|| name_rest.clone());
    let title = if let Some((_a, t)) = maybe_split_artist_title(&title0,
        if artist.is_empty() {
        None
    } else {
        Some(&artist)
    }) {
        normalize_ws(strip_tech_brackets(t))
    } else {
        normalize_ws(strip_tech_brackets(title0))
    };

    let track_no = tf.track_no.or(track_from_name);
    let disc_no = tf.disc_no.or(disc_from_name).or(disc_from_path);

    TrackMeta {
        artist,
        album,
        title,
        track_no,
        disc_no,
    }
}
