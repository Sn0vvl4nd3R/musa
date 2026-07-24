use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use crate::app::Theme;

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
