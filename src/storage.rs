use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use crate::app::Theme;

#[derive(Clone, Debug)]
pub struct StoredPlaylist {
    pub name: String,
    pub tracks: Vec<PathBuf>,
}

pub fn load_roots() -> Vec<PathBuf> {
    let Ok(text) = fs::read_to_string(config_dir().join("libraries.txt")) else {
        return Vec::new();
    };

    let mut roots: Vec<PathBuf> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .collect();

    roots.sort();
    roots.dedup();
    roots
}

pub fn save_roots(roots: &[PathBuf]) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;

    let mut text = String::new();
    for root in roots {
        text.push_str(&root.to_string_lossy());
        text.push('\n');
    }
    fs::write(dir.join("libraries.txt"), text)
}

pub fn load_theme() -> Theme {
    match fs::read_to_string(config_dir().join("theme")) {
        Ok(value) if value.trim().eq_ignore_ascii_case("light") => Theme::Light,
        _ => Theme::Dark,
    }
}

pub fn save_theme(theme: Theme) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("theme"),
        match theme {
            Theme::Dark => "dark\n",
            Theme::Light => "light\n",
        },
    )
}

pub fn load_playlists() -> Vec<StoredPlaylist> {
    let Ok(text) = fs::read_to_string(config_dir().join("playlists.txt")) else {
        return Vec::new();
    };

    let mut playlists = Vec::new();
    let mut current: Option<StoredPlaylist> = None;

    for line in text.lines() {
        if line.is_empty() {
            if let Some(playlist) = current.take() {
                push_valid_playlist(&mut playlists, playlist);
            }
            continue;
        }

        let Some((kind, value)) = line.split_once('\t') else {
            continue;
        };
        let value = unescape_field(value);

        match kind {
            "P" => {
                if let Some(playlist) = current.take() {
                    push_valid_playlist(&mut playlists, playlist);
                }
                current = Some(StoredPlaylist {
                    name: value,
                    tracks: Vec::new(),
                });
            }
            "T" => {
                if let Some(playlist) = current.as_mut() {
                    playlist.tracks.push(PathBuf::from(value));
                }
            }
            _ => {}
        }
    }

    if let Some(playlist) = current {
        push_valid_playlist(&mut playlists, playlist);
    }

    playlists
}

pub fn save_playlists(playlists: &[StoredPlaylist]) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;

    let mut text = String::new();
    for playlist in playlists {
        text.push_str("P\t");
        text.push_str(&escape_field(&playlist.name));
        text.push('\n');
        for path in &playlist.tracks {
            text.push_str("T\t");
            text.push_str(&escape_field(&path.to_string_lossy()));
            text.push('\n');
        }
        text.push('\n');
    }

    fs::write(dir.join("playlists.txt"), text)
}

fn push_valid_playlist(playlists: &mut Vec<StoredPlaylist>, mut playlist: StoredPlaylist) {
    playlist.name = playlist.name.trim().to_owned();
    if playlist.name.is_empty()
        || playlists
            .iter()
            .any(|existing| existing.name.eq_ignore_ascii_case(&playlist.name))
    {
        return;
    }

    let mut unique = Vec::with_capacity(playlist.tracks.len());
    for path in playlist.tracks {
        if !unique.iter().any(|existing| existing == &path) {
            unique.push(path);
        }
    }
    playlist.tracks = unique;
    playlists.push(playlist);
}

fn escape_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn unescape_field(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }

        match characters.next() {
            Some('t') => result.push('\t'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    result
}

pub fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_dir() -> PathBuf {
    if let Some(path) = env::var_os("MUSA_CONFIG_DIR") {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(path).join("musa");
    }
    if let Some(path) = env::var_os("APPDATA") {
        return PathBuf::from(path).join("musa");
    }
    home_dir().join(Path::new(".config")).join("musa")
}
