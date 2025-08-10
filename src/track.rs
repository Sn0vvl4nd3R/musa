use std::path::{
    Path,
    PathBuf
};
use crate::util::is_audio_file;
use anyhow::anyhow;
use walkdir::WalkDir;


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
        let m = crate::util::parse_track_meta(&path);

        let title = if m.title.is_empty() {
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string()
        } else {
            m.title
        };
        let artist = if m.artist.is_empty() {
            "Unknown Artist".into()
        } else {
            m.artist
        };
        let album  = m.album;

        let album_dir: PathBuf = path
            .parent()
            .and_then(|p| {
                if p.file_name().is_some_and(|n| n == "songs") {
                    p.parent()
                } else {
                    Some(p)
                }
            })
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        out.push(Track {
            path,
            title,
            artist,
            album,
            album_dir,
            track_no: m.track_no,
            disc_no:  m.disc_no,
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
