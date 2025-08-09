use anyhow::anyhow;
use std::path::{
    Path,
    PathBuf
};
use walkdir::WalkDir;

use crate::util::{
    is_audio_file,
    clean_album_for_display,
};
use regex::Regex;

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_dir: PathBuf,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
}

impl Track {
    pub fn display_line(&self) -> String {
        self.title.clone()
    }
    pub fn display_now_playing(&self) -> (String, String, String) {
        (self.title.clone(), self.artist.clone(), self.album.clone())
    }
}

fn parse_disc_folder(name: &str) -> Option<u32> {
    let re = Regex::new(r"(?i)^(?:cd|disc)\s*[-_ ]?\s*(\d{1,3})$").unwrap();
    re.captures(name.trim())
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

fn album_dir_and_disc(file: &Path) -> (PathBuf, Option<u32>) {
    let mut dir = file.parent().unwrap_or_else(|| Path::new("/"));
    let mut disc = None;
    if let Some(name) = dir.file_name().and_then(|s| s.to_str()) {
        if let Some(d) = parse_disc_folder(name) {
            disc = Some(d);
            dir = dir.parent().unwrap_or(dir);
        }
    }
    (dir.to_path_buf(), disc)
}

pub fn guess_artist_album(file: &Path) -> (String, String, PathBuf, Option<u32>) {
    let (album_dir, disc_no) = album_dir_and_disc(file);
    let album_name_raw = album_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let artist = album_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let album = clean_album_for_display(album_name_raw, &artist);
    (artist, album, album_dir, disc_no)
}

pub fn strip_leading_track_numbers(stem: &str) -> (Option<u32>, String) {
    let mut s = stem.to_string();
    let mut first: Option<u32> = None;
    let re_delim = Regex::new(r#"^\s*(\d{1,3})\s*([)\]\-._]+)\s*"#).unwrap();
    let re_zero_space = Regex::new(r#"^\s*(0\d{1,2})\s+"#).unwrap();
    let re_cleanup = Regex::new(r#"^\s*[)\]\-._]+\s*"#).unwrap();
    loop {
        if let Some(cap) = re_delim.captures(&s) {
            if first.is_none() {
                if let Ok(n) = cap[1].parse::<u32>() {
                    first = Some(n);
                }
            }
            let end = cap.get(0).unwrap().end();
            s = s[end..].to_string();
            loop {
                if let Some(m) = re_cleanup.find(&s) {
                    if m.start() == 0 {
                        s = s[m.end()..].to_string();
                        continue;
                    }
                }
                break;
            }
            continue;
        }
        if let Some(cap) = re_zero_space.captures(&s) {
            if first.is_none() {
                if let Ok(n) = cap[1].parse::<u32>() {
                    first = Some(n);
                }
            }
            let end = cap.get(0).unwrap().end();
            s = s[end..].to_string();
            continue;
        }
        break;
    }
    (first, s.trim().to_string())
}

pub fn split_artist_title(s: &str) -> (Option<String>, String) {
    for sep in [" - ", " – ", " — "] {
        if let Some(idx) = s.find(sep) {
            let artist = s[..idx].trim();
            let title = s[idx + sep.len()..].trim();
            if !artist.is_empty() && !title.is_empty() {
                return (Some(artist.to_string()), title.to_string());
            }
        }
    }
    (None, s.trim().to_string())
}

pub fn scan_tracks(root: &Path) -> anyhow::Result<Vec<Track>> {
    if !root.exists() {
        return Err(anyhow!("Path does not exist: {}", root.display()));
    }
    let mut files: Vec<PathBuf> = Vec::new();
    if root.is_file() {
        if is_audio_file(root) {
            files.push(root.canonicalize()?);
        }
    } else {
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() && is_audio_file(p) {
                files.push(p.canonicalize()?);
            }
        }
    }
    if files.is_empty() {
        return Err(anyhow!("No audio files in {}", root.display()));
    }
    files.sort();

    let mut out = Vec::with_capacity(files.len());
    for path in files {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let stem = crate::util::normalize_ws(stem);
        let (leading_no, s1) = strip_leading_track_numbers(&stem);
        let (artist_from_file, s2) = split_artist_title(&s1);
        let title = crate::util::normalize_ws(s2);
        let (artist_from_dir, album, album_dir, disc_no) = guess_artist_album(&path);
        let artist = artist_from_file.unwrap_or_else(|| artist_from_dir.clone());
        out.push(Track {
            path,
            title,
            artist,
            album,
            album_dir,
            track_no: leading_no,
            disc_no,
        });
    }

    out.sort_by(|a, b| {
        let ad = a.disc_no.unwrap_or(0);
        let bd = b.disc_no.unwrap_or(0);
        match ad.cmp(&bd) {
            std::cmp::Ordering::Equal => {
                let at = a.track_no.unwrap_or(10_000);
                let bt = b.track_no.unwrap_or(10_000);
                match at.cmp(&bt) {
                    std::cmp::Ordering::Equal => a.path.cmp(&b.path),
                    x => x,
                }
            }
            x => x,
        }
    });

    Ok(out)
}
