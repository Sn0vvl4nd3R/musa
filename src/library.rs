use std::{
    collections::HashSet,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender},
    },
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
    pub album_dir: Arc<Path>,
    pub title: Arc<str>,
    pub artist: Arc<str>,
    pub album_artist: Arc<str>,
    pub album: Arc<str>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub duration: Option<Duration>,
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

        Self {
            path,
            album_dir: Arc::from(fallback.album_dir.into_boxed_path()),
            title: Arc::from(title),
            artist: Arc::from(artist),
            album_artist: Arc::from(album_artist),
            album: Arc::from(album),
            track_no: track_no.or(fallback.track_no),
            disc_no: disc_no.or(fallback.disc_no),
            duration,
        }
    }
}

#[derive(Debug)]
pub enum ScanEvent {
    Progress { done: usize, total: usize },
    Finished(Result<Vec<Track>, String>),
}

pub fn spawn_scan(roots: Vec<PathBuf>) -> Receiver<ScanEvent> {
    let (sender, receiver) = mpsc::sync_channel(4);
    let worker_sender = sender.clone();
    let spawn = thread::Builder::new()
        .name("musa-library-scan".to_owned())
        .stack_size(512 * 1024)
        .spawn(move || {
            let result = scan(&roots, &worker_sender).map_err(|error| error.to_string());
            let _ = worker_sender.send(ScanEvent::Finished(result));
        });

    if let Err(error) = spawn {
        let _ = sender.send(ScanEvent::Finished(Err(format!(
            "failed to start library scan: {error}"
        ))));
    }
    receiver
}

fn scan(roots: &[PathBuf], sender: &SyncSender<ScanEvent>) -> io::Result<Vec<Track>> {
    let mut paths = collect_audio_paths(roots)?;
    paths.sort_unstable();
    paths.dedup();

    let total = paths.len();
    let mut tracks = Vec::with_capacity(total);
    for (index, path) in paths.into_iter().enumerate() {
        tracks.push(Track::from_path(path));
        let done = index + 1;
        if done == total || done % 64 == 0 {
            let _ = sender.try_send(ScanEvent::Progress { done, total });
        }
    }

    intern_repeated_metadata(&mut tracks);

    tracks.sort_by_cached_key(|track| {
        (
            track.artist.to_lowercase(),
            track.album.to_lowercase(),
            track.disc_no.unwrap_or(0),
            track.track_no.unwrap_or(u32::MAX),
            track.title.to_lowercase(),
            track.path.clone(),
        )
    });

    Ok(tracks)
}

fn intern_repeated_metadata(tracks: &mut [Track]) {
    let mut strings = HashSet::<Arc<str>>::with_capacity(tracks.len().saturating_mul(2));
    let mut directories = HashSet::<Arc<Path>>::with_capacity((tracks.len() / 8).max(8));
    for track in tracks {
        intern_arc(&mut strings, &mut track.artist);
        intern_arc(&mut strings, &mut track.album_artist);
        intern_arc(&mut strings, &mut track.album);
        intern_path(&mut directories, &mut track.album_dir);
    }
}

fn intern_arc(pool: &mut HashSet<Arc<str>>, value: &mut Arc<str>) {
    if let Some(existing) = pool.get(value.as_ref()) {
        *value = Arc::clone(existing);
    } else {
        pool.insert(Arc::clone(value));
    }
}

fn intern_path(pool: &mut HashSet<Arc<Path>>, value: &mut Arc<Path>) {
    if let Some(existing) = pool.get(value.as_ref()) {
        *value = Arc::clone(existing);
    } else {
        pool.insert(Arc::clone(value));
    }
}

fn collect_audio_paths(roots: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let mut output = Vec::new();
    let mut pending = Vec::with_capacity(roots.len().max(16));
    pending.extend(roots.iter().cloned());

    while let Some(path) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
                ) =>
            {
                continue;
            }
            Err(error) => return Err(error),
        };

        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_file() {
            if is_supported_audio(&path) {
                output.push(path);
            }
            continue;
        }
        if !file_type.is_dir() {
            continue;
        }

        let read_dir = match fs::read_dir(&path) {
            Ok(read_dir) => read_dir,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => continue,
            Err(error) => return Err(error),
        };

        for entry in read_dir.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let child = entry.path();
            if file_type.is_dir() {
                pending.push(child);
            } else if file_type.is_file() && is_supported_audio(&child) {
                output.push(child);
            }
        }
    }

    Ok(output)
}

pub fn read_directory_entries(path: &Path) -> io::Result<Vec<DirectoryEntry>> {
    let read_dir = fs::read_dir(path)?;
    let mut entries = Vec::new();

    for entry in read_dir.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            entries.push(DirectoryEntry {
                name,
                kind: DirectoryEntryKind::Directory(path),
            });
        } else if file_type.is_file() && is_supported_audio(&path) {
            let track = Track::from_path(path.clone());
            entries.push(DirectoryEntry {
                name,
                kind: DirectoryEntryKind::Track(track),
            });
        }
    }

    entries.sort_by_cached_key(|entry| match &entry.kind {
        DirectoryEntryKind::Directory(_) => (0, 0, 0, entry.name.to_lowercase()),
        DirectoryEntryKind::Track(track) => (
            1,
            track.disc_no.unwrap_or(0),
            track.track_no.unwrap_or(u32::MAX),
            track.title.to_lowercase(),
        ),
    });
    Ok(entries)
}

#[derive(Clone, Debug)]
pub enum DirectoryEntryKind {
    Directory(PathBuf),
    Track(Track),
}

#[derive(Clone, Debug)]
pub struct DirectoryEntry {
    pub name: String,
    pub kind: DirectoryEntryKind,
}

pub fn is_supported_audio(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };

    ["flac", "mp3", "ogg", "oga", "wav", "m4a", "m4b", "mp4", "aac"]
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
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
        let Some(rest) = lower.strip_prefix(prefix) else {
            continue;
        };
        let bytes = rest.as_bytes();
        let mut start = 0;
        while start < bytes.len() && !bytes[start].is_ascii_digit() {
            start += 1;
        }
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if start < end {
            return rest[start..end].parse().ok();
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
    if value.len() >= 5 {
        let bytes = value.as_bytes();
        if bytes[..4].iter().all(|byte| byte.is_ascii_digit())
            && matches!(bytes[4], b' ' | b'-' | b'.' | b'_')
        {
            value = value[5..].trim_start().to_owned();
        }
    }

    let mut cleaned = String::with_capacity(value.len());
    for word in value.split_whitespace() {
        if !cleaned.is_empty() {
            cleaned.push(' ');
        }
        cleaned.push_str(word);
    }
    cleaned
}
