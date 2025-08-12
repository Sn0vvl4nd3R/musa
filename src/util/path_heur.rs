use regex::Regex;
use once_cell::sync::Lazy;
use std::path::{
    Path,
    Component
};

const BUCKETS: &[&str] = &[
    "albums", "album", "singles", "single", "eps", "ep", "compilations", "compilation",
    "records", "releases", "collections", "library", "audio", "music", "media",
    "disc", "discs", "disk", "cd", "cds", "cd1", "cd2", "cd3", "dvd",
    "tracks", "track", "songs", "song",
];

#[inline]
pub(crate) fn is_bucket(name: &str) -> bool {
    BUCKETS.iter().any(|b| name.eq_ignore_ascii_case(b))
}

static RE_DISC_LABEL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:cd|disc|disk)\s*-?\s*(\d{1,2})\b").unwrap());

#[inline]
pub(crate) fn parse_disc_label(s: &str) -> Option<u32> {
    RE_DISC_LABEL
        .captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

pub(crate) fn infer_artist_album_from_path(p: &Path) -> (Option<String>, Option<String>, Option<u32>) {
    let mut parts: Vec<String> = Vec::with_capacity(p.components().count());
    for c in p.components() {
        if let Component::Normal(os) = c {
            parts.push(os.to_string_lossy().into_owned());
        }
    }

    let mut cand: Vec<(usize, String)> = Vec::with_capacity(parts.len());
    for (i, name) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let stem = if last {
            std::path::Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
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
        let artist = &cand[cand.len() - 3].1;
        return (Some(artist.clone()), Some(album_raw.clone()), disc_no);
    }

    (None, None, disc_no)
}
