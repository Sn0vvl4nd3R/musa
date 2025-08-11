use regex::Regex;
use std::path::Path;

use super::{
    path_heur::infer_artist_album_from_path,
    tags::read_tags,
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

    let album = if artist.is_empty() {
        normalize_ws(album_raw)
    } else {
        clean_album_for_display(&album_raw, &artist)
    };

    let title0 = tf.title.clone().unwrap_or_else(|| name_rest.clone());
    let title = if let Some((_a, t)) = maybe_split_artist_title(
        &title0,
        if artist.is_empty() {
            None
        } else {
            Some(&artist)
        }
    ) {
        normalize_ws(strip_tech_brackets(t))
    } else {
        normalize_ws(strip_tech_brackets(title0))
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
