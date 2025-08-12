use regex::Regex;
use std::path::Path;
use std::borrow::Cow;
use once_cell::sync::Lazy;

use super::{
    tags::read_tags,
    path_heur::infer_artist_album_from_path,
    text::{
        normalize_ws,
        strip_tech_brackets,
        clean_album_for_display
    },
};

#[derive(Debug, Clone)]
pub struct TrackMeta {
    pub artist: String,
    pub album: String,
    pub title: String,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
}

static RE_DISC_TRACK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?x)^ \s*
        (?P<disc>\d{1,2}) \s* [-_.] \s*
        (?P<track>\d{1,3})
        (?:\s* [\.\)\-_] \s*){1,3} \s+
        (?P<rest>.+)
    $"#).unwrap()
});

static RE_TRACK_REST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?x)^ \s*
        (?P<track>\d{1,3})
        (?:\s* [\.\)\-_] \s*){1,3} \s+
        (?P<rest>.+)
    $"#).unwrap()
});

static RE_ONLY_TRACK: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^\s*(?P<track>\d{1,3})\s*$"#).unwrap());

#[inline]
fn trim_leading_separators(mut s: &str) -> &str {
    for sep in ["- ", "– ", "— "] {
        if let Some(rest) = s.strip_prefix(sep) {
            s = rest;
            break;
        }
    }
    s.trim_start()
}

fn parse_track_number(s: &str) -> (Option<u32>, Option<u32>, String) {
    if let Some(c) = RE_DISC_TRACK.captures(s) {
        let disc = c["disc"].parse::<u32>().ok();
        let track = c["track"].parse::<u32>().ok();
        let rest = trim_leading_separators(c.name("rest").unwrap().as_str()).to_string();
        return (track, disc, rest);
    }
    if let Some(c) = RE_TRACK_REST.captures(s) {
        let track = c["track"].parse::<u32>().ok();
        let rest = trim_leading_separators(c.name("rest").unwrap().as_str()).to_string();
        return (track, None, rest);
    }
    if let Some(c) = RE_ONLY_TRACK.captures(s) {
        let track = c["track"].parse::<u32>().ok();
        return (track, None, String::new());
    }
    (None, None, s.trim().to_string())
}

fn maybe_split_artist_title(file_stem: &str, known_artist: Option<&str>) -> Option<(String, String)> {
    for sep in [" - ", " – ", " — "] {
        if let Some(idx) = file_stem.find(sep) {
            if let Some(ka) = known_artist {
                let (left, right) = file_stem.split_at(idx);
                let right = &right[sep.len()..];
                if left.trim().eq_ignore_ascii_case(ka.trim()) {
                    return Some((left.trim().to_string(), right.trim().to_string()));
                }
            }
        }
    }
    None
}

pub fn parse_track_meta(path: &Path) -> TrackMeta {
    let tf = read_tags(path).unwrap_or_default();

    let (artist_from_path, album_from_path_raw, disc_from_path) = infer_artist_album_from_path(path);

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let (track_from_name, disc_from_name, name_rest) = parse_track_number(stem);

    let artist = tf.artist.as_deref()
        .or(tf.album_artist.as_deref())
        .or(artist_from_path.as_deref())
        .unwrap_or("")
        .to_owned();

    let album_raw = tf.album.as_deref()
        .or(album_from_path_raw.as_deref())
        .unwrap_or("")
        .to_owned();

    let album = if artist.is_empty() {
        normalize_ws(&album_raw).into_owned()
    } else {
        clean_album_for_display(&album_raw, &artist)
    };

    let title0_cow: Cow<'_, str> = if let Some(t) = tf.title.as_deref() {
        Cow::Borrowed(t)
    } else {
        Cow::Owned(name_rest)
    };

    let title = if let Some((_a, t)) = maybe_split_artist_title(&title0_cow, if artist.is_empty() {
        None
    } else {
        Some(&artist)
    }) {
        normalize_ws(&strip_tech_brackets(&t)).into_owned()
    } else {
        normalize_ws(&strip_tech_brackets(&title0_cow)).into_owned()
    };

    let track_no = tf.track_no.or(track_from_name);
    let disc_no  = tf.disc_no.or(disc_from_name).or(disc_from_path);

    TrackMeta {
        artist,
        album,
        title,
        track_no,
        disc_no
    }
}
