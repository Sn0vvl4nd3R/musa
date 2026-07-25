use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use lofty::{
    file::{AudioFile, TaggedFileExt},
    read_from_path,
    tag::{Accessor, ItemKey},
};

#[derive(Clone, Debug)]
pub struct Track {
    pub path: PathBuf,
    pub album_dir: PathBuf,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub duration: Option<Duration>,
    pub search_text: String,
}

impl Track {
    pub(crate) fn from_path(path: PathBuf) -> Self {
        let fallback = FallbackMeta::from_path(&path);

        let mut title = None;
        let mut artist = None;
        let mut album_artist = None;
        let mut album = None;
        let mut track_no = None;
        let mut disc_no = None;
        let mut duration = None;

        if let Ok(tagged) = read_from_path(&path) {
            let parsed_duration = tagged.properties().duration();
            if !parsed_duration.is_zero() {
                duration = Some(parsed_duration);
            }

            if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
                title = tag.title().map(|value| value.trim().to_owned());
                artist = tag.artist().map(|value| value.trim().to_owned());
                album = tag.album().map(|value| value.trim().to_owned());
                album_artist = tag
                    .get_string(ItemKey::AlbumArtist)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
                track_no = tag.track();
                disc_no = tag.disk();
            }
        }

        let title = non_empty(title).unwrap_or(fallback.title);
        let artist = non_empty(artist).unwrap_or(fallback.artist);
        let album = non_empty(album).unwrap_or(fallback.album);
        let album_artist = non_empty(album_artist).unwrap_or_else(|| artist.clone());
        let track_no = track_no.or(fallback.track_no);
        let disc_no = disc_no.or(fallback.disc_no);
        let album_dir = fallback.album_dir;

        let search_text = format!(
            "{}\n{}\n{}\n{}\n{}",
            title,
            artist,
            album_artist,
            album,
            path.to_string_lossy()
        )
        .to_lowercase();

        Self {
            path,
            album_dir,
            title,
            artist,
            album_artist,
            album,
            track_no,
            disc_no,
            duration,
            search_text,
        }
    }
}

#[derive(Debug)]
pub enum ScanEvent {
    Progress { done: usize, total: usize },
    Finished(Result<Vec<Track>, String>),
}

pub fn spawn_scan(roots: Vec<PathBuf>) -> Receiver<ScanEvent> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = scan(&roots, &sender).map_err(|error| error.to_string());
        let _ = sender.send(ScanEvent::Finished(result));
    });
    receiver
}

fn scan(roots: &[PathBuf], sender: &Sender<ScanEvent>) -> io::Result<Vec<Track>> {
    let mut paths = Vec::new();
    for root in roots {
        collect(root, &mut paths)?;
    }

    paths.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
    paths.dedup();

    let total = paths.len();
    let mut tracks = Vec::with_capacity(total);
    for (index, path) in paths.into_iter().enumerate() {
        tracks.push(Track::from_path(path));
        let done = index + 1;
        if done == total || done % 16 == 0 {
            let _ = sender.send(ScanEvent::Progress { done, total });
        }
    }

    tracks.sort_by(|left, right| {
        lower(&left.artist)
            .cmp(&lower(&right.artist))
            .then_with(|| lower(&left.album).cmp(&lower(&right.album)))
            .then_with(|| left.disc_no.unwrap_or(0).cmp(&right.disc_no.unwrap_or(0)))
            .then_with(|| {
                left.track_no
                    .unwrap_or(u32::MAX)
                    .cmp(&right.track_no.unwrap_or(u32::MAX))
            })
            .then_with(|| lower(&left.title).cmp(&lower(&right.title)))
            .then_with(|| left.path.cmp(&right.path))
    });

    Ok(tracks)
}

fn collect(path: &Path, output: &mut Vec<PathBuf>) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied) => {
            return Ok(())
        }
        Err(error) => return Err(error),
    };

    if metadata.file_type().is_symlink() {
        return Ok(());
    }

    if metadata.is_file() {
        if is_supported_audio(path) {
            output.push(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
        }
        return Ok(());
    }

    if !metadata.is_dir() {
        return Ok(());
    }

    let read_dir = match fs::read_dir(path) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return Ok(()),
        Err(error) => return Err(error),
    };

    let mut entries: Vec<_> = read_dir.filter_map(Result::ok).collect();
    entries.sort_by_cached_key(|entry| entry.file_name().to_string_lossy().to_lowercase());

    for entry in entries {
        collect(&entry.path(), output)?;
    }

    Ok(())
}

pub fn read_directory_entries(path: &Path) -> io::Result<Vec<DirectoryEntry>> {
    let read_dir = fs::read_dir(path)?;
    let mut entries = Vec::new();

    for entry in read_dir.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() && !file_type.is_symlink() {
            entries.push(DirectoryEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path(),
                kind: DirectoryEntryKind::Directory,
            });
        } else if file_type.is_file() && is_supported_audio(&entry.path()) {
            let path = entry.path();
            let track = Track::from_path(path.clone());
            entries.push(DirectoryEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path,
                kind: DirectoryEntryKind::Track(track),
            });
        }
    }

    entries.sort_by(|left, right| {
        left.kind
            .sort_rank()
            .cmp(&right.kind.sort_rank())
            .then_with(|| match (&left.kind, &right.kind) {
                (DirectoryEntryKind::Track(left), DirectoryEntryKind::Track(right)) => left
                    .disc_no
                    .unwrap_or(0)
                    .cmp(&right.disc_no.unwrap_or(0))
                    .then_with(|| {
                        left.track_no
                            .unwrap_or(u32::MAX)
                            .cmp(&right.track_no.unwrap_or(u32::MAX))
                    })
                    .then_with(|| lower(&left.title).cmp(&lower(&right.title))),
                _ => lower(&left.name).cmp(&lower(&right.name)),
            })
    });
    Ok(entries)
}

#[derive(Clone, Debug)]
pub enum DirectoryEntryKind {
    Directory,
    Track(Track),
}

impl DirectoryEntryKind {
    fn sort_rank(&self) -> u8 {
        match self {
            Self::Directory => 0,
            Self::Track(_) => 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: DirectoryEntryKind,
}

pub fn is_supported_audio(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "flac" | "mp3" | "ogg" | "oga" | "wav" | "m4a" | "m4b" | "mp4" | "aac"
    )
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn lower(value: &str) -> String {
    value.to_lowercase()
}

struct FallbackMeta {
    album_dir: PathBuf,
    title: String,
    artist: String,
    album: String,
    track_no: Option<u32>,
    disc_no: Option<u32>,
}

impl FallbackMeta {
    fn from_path(path: &Path) -> Self {
        let stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("Unknown track")
            .trim();
        let (name_disc, name_track, cleaned_stem) = parse_leading_numbers(stem);

        let (file_artist, file_title) = split_artist_title(cleaned_stem)
            .map(|(artist, title)| (Some(artist.to_owned()), title.to_owned()))
            .unwrap_or((None, cleaned_stem.to_owned()));

        let direct_parent = path.parent().unwrap_or_else(|| Path::new("."));
        let direct_parent_name = file_name(direct_parent);
        let parent_disc = parse_disc_label(&direct_parent_name);

        let album_dir = if parent_disc.is_some() || is_bucket(&direct_parent_name) {
            direct_parent.parent().unwrap_or(direct_parent).to_path_buf()
        } else {
            direct_parent.to_path_buf()
        };

        let album = clean_directory_name(&file_name(&album_dir));
        let inferred_artist = album_dir
            .parent()
            .map(file_name)
            .filter(|value| !value.is_empty() && !is_bucket(value))
            .map(|value| clean_directory_name(&value));

        Self {
            album_dir,
            title: if file_title.trim().is_empty() {
                "Unknown track".to_owned()
            } else {
                file_title.trim().to_owned()
            },
            artist: file_artist
                .or(inferred_artist)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Unknown Artist".to_owned()),
            album: if album.trim().is_empty() {
                "Unknown Album".to_owned()
            } else {
                album
            },
            track_no: name_track,
            disc_no: name_disc.or(parent_disc),
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_owned()
}

fn split_artist_title(input: &str) -> Option<(&str, &str)> {
    for separator in [" - ", " – ", " — "] {
        if let Some((artist, title)) = input.split_once(separator) {
            let artist = artist.trim();
            let title = title.trim();
            if !artist.is_empty() && !title.is_empty() {
                return Some((artist, title));
            }
        }
    }
    None
}

fn parse_leading_numbers(input: &str) -> (Option<u32>, Option<u32>, &str) {
    let bytes = input.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        cursor += 1;
    }
    if cursor == 0 {
        return (None, None, input.trim());
    }

    let first = input[..cursor].parse::<u32>().ok();
    let separator_start = cursor;
    while cursor < bytes.len() && matches!(bytes[cursor], b'.' | b'-' | b'_' | b' ') {
        cursor += 1;
    }

    let second_start = cursor;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        cursor += 1;
    }
    let has_second = cursor > second_start;

    let (disc, track) = if has_second && second_start > separator_start {
        (first, input[second_start..cursor].parse::<u32>().ok())
    } else {
        cursor = separator_start;
        (None, first)
    };

    while cursor < bytes.len()
        && matches!(bytes[cursor], b' ' | b'.' | b'-' | b'_' | b')' | b']')
    {
        cursor += 1;
    }

    let rest = input.get(cursor..).unwrap_or(input).trim();
    if rest.is_empty() {
        (disc, track, input.trim())
    } else {
        (disc, track, rest)
    }
}

fn parse_disc_label(input: &str) -> Option<u32> {
    let lower = input.to_ascii_lowercase();
    for prefix in ["cd", "disc", "disk"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let digits: String = rest
                .chars()
                .skip_while(|ch| !ch.is_ascii_digit())
                .take_while(|ch| ch.is_ascii_digit())
                .collect();
            if !digits.is_empty() {
                return digits.parse().ok();
            }
        }
    }
    None
}

fn is_bucket(input: &str) -> bool {
    const BUCKETS: &[&str] = &[
        "music", "audio", "media", "songs", "song", "tracks", "track", "albums", "album",
        "library", "releases", "release", "discs", "disc", "cds", "cd",
    ];
    BUCKETS.iter().any(|name| input.eq_ignore_ascii_case(name))
}

fn clean_directory_name(input: &str) -> String {
    let mut value = input.replace('_', " ");
    if value.len() >= 7 {
        let bytes = value.as_bytes();
        if bytes[..4].iter().all(|byte| byte.is_ascii_digit())
            && matches!(bytes[4], b' ' | b'-' | b'.' | b'_')
        {
            value = value[5..].trim_start().to_owned();
        }
    }
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
