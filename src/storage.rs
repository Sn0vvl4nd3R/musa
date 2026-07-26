use std::{
    collections::HashSet,
    env, fs,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::app::Theme;

#[derive(Clone, Debug)]
pub struct StoredPlaylist {
    pub name: String,
    pub tracks: Vec<PathBuf>,
}

pub fn load_roots() -> Vec<PathBuf> {
    let Ok(file) = File::open(config_dir().join("libraries.txt")) else {
        return Vec::new();
    };

    let mut roots: Vec<PathBuf> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .collect();

    roots.sort_unstable();
    roots.dedup();
    roots
}

pub fn save_roots(roots: &[PathBuf]) -> io::Result<()> {
    atomic_write("libraries.txt", |writer| {
        for root in roots {
            writeln!(writer, "{}", root.display())?;
        }
        Ok(())
    })
}

pub fn load_theme() -> Theme {
    match fs::read_to_string(config_dir().join("theme")) {
        Ok(value) if value.trim().eq_ignore_ascii_case("light") => Theme::Light,
        _ => Theme::Dark,
    }
}

pub fn save_theme(theme: Theme) -> io::Result<()> {
    atomic_write("theme", |writer| {
        let value: &[u8] = match theme {
            Theme::Dark => b"dark\n",
            Theme::Light => b"light\n",
        };
        writer.write_all(value)
    })
}

pub fn load_playlists() -> Vec<StoredPlaylist> {
    let Ok(file) = File::open(config_dir().join("playlists.txt")) else {
        return Vec::new();
    };

    let mut playlists = Vec::new();
    let mut current: Option<StoredPlaylist> = None;

    for line in BufReader::new(file).lines().map_while(Result::ok) {
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

pub fn save_playlists<'a, I>(playlists: I) -> io::Result<()>
where
    I: IntoIterator<Item = (&'a str, &'a [PathBuf])>,
{
    atomic_write("playlists.txt", |writer| {
        let mut escaped = String::new();
        for (name, tracks) in playlists {
            writer.write_all(b"P\t")?;
            escape_field_into(name, &mut escaped);
            writer.write_all(escaped.as_bytes())?;
            writer.write_all(b"\n")?;

            for path in tracks {
                writer.write_all(b"T\t")?;
                escape_field_into(&path.to_string_lossy(), &mut escaped);
                writer.write_all(escaped.as_bytes())?;
                writer.write_all(b"\n")?;
            }
            writer.write_all(b"\n")?;
        }
        Ok(())
    })
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

    let mut seen = HashSet::with_capacity(playlist.tracks.len());
    playlist.tracks.retain(|path| seen.insert(path.clone()));
    playlists.push(playlist);
}

fn escape_field_into(value: &str, output: &mut String) {
    output.clear();
    output.reserve(value.len());
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            other => output.push(other),
        }
    }
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

fn atomic_write(
    file_name: &str,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;

    let target = dir.join(file_name);
    let temporary = dir.join(format!(".{file_name}.tmp"));
    let file = File::create(&temporary)?;
    let mut writer = BufWriter::new(file);
    write(&mut writer)?;
    writer.flush()?;
    drop(writer);

    #[cfg(windows)]
    if target.exists() {
        let _ = fs::remove_file(&target);
    }
    fs::rename(temporary, target)
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
