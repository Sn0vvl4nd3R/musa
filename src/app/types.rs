use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiView {
    Settings,
    Browser,
    Player,
    Playlist,
}

#[derive(Clone)]
pub struct DirEntryItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}
