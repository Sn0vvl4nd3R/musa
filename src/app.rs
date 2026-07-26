use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{Receiver, TryRecvError},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    Result,
    audio::AudioEngine,
    library::{self, DirectoryEntry, DirectoryEntryKind, ScanEvent, Track},
    storage,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::All => "all",
            Self::One => "one",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Home,
    Search,
    Songs,
    Albums,
    Artists,
    Playlists,
    Folders,
}

impl View {
    pub const ALL: [Self; 7] = [
        Self::Home,
        Self::Search,
        Self::Songs,
        Self::Albums,
        Self::Artists,
        Self::Playlists,
        Self::Folders,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Search => "Search",
            Self::Songs => "Songs",
            Self::Albums => "Albums",
            Self::Artists => "Artists",
            Self::Playlists => "Playlists",
            Self::Folders => "Folders",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailView {
    Album(usize),
    Artist(usize),
    Playlist(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderFocus {
    Roots,
    Browser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchItem {
    Playlist(usize),
    Artist(usize),
    Album(usize),
    Track(usize),
}

#[derive(Clone, Debug)]
pub struct Album {
    pub title: Arc<str>,
    pub artist: Arc<str>,
    pub tracks: Vec<usize>,
    pub duration: Duration,
}

#[derive(Clone, Debug)]
pub struct Artist {
    pub name: Arc<str>,
    pub tracks: Vec<usize>,
    pub album_count: usize,
}


#[derive(Clone, Debug)]
pub struct Playlist {
    pub name: String,
    pub track_paths: Vec<PathBuf>,
    pub tracks: Vec<usize>,
    pub duration: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextInputKind {
    CreatePlaylist,
    RenamePlaylist(usize),
}

#[derive(Clone, Debug)]
pub struct TextInput {
    pub prompt: String,
    pub value: String,
    pub kind: TextInputKind,
    pub pending_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct PlaylistPicker {
    pub selected: usize,
    pub track_paths: Vec<PathBuf>,
    pub source_label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanPhase {
    Idle,
    Discovering,
    Reading { done: usize, total: usize },
}

pub struct App {
    pub roots: Vec<PathBuf>,
    pub tracks: Vec<Track>,
    path_order: Vec<usize>,
    recent_indices_cache: Vec<usize>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,

    pub view: View,
    pub detail: Option<DetailView>,
    pub selected: usize,

    pub search_query: String,
    pub search_results: Vec<SearchItem>,
    pub search_editing: bool,

    pub current: Option<usize>,
    pub state: PlaybackState,
    pub volume: u8,
    pub shuffle: bool,
    pub repeat: RepeatMode,

    pub queue_base: Vec<usize>,
    pub queue: Vec<usize>,
    pub queue_pos: Option<usize>,
    pub recent_paths: Vec<PathBuf>,

    pub status: String,
    pub theme: Theme,
    pub help_open: bool,
    pub text_input: Option<TextInput>,
    pub playlist_picker: Option<PlaylistPicker>,

    pub browser_dir: PathBuf,
    pub browser_entries: Vec<DirectoryEntry>,
    pub folder_focus: FolderFocus,
    pub root_selected: usize,
    pub browser_selected: usize,

    browser_queue_base: Vec<PathBuf>,
    browser_queue: Vec<PathBuf>,
    browser_queue_pos: Option<usize>,
    browser_current: Option<Track>,

    pub scan_phase: ScanPhase,
    scan_rx: Option<Receiver<ScanEvent>>,
    rescan_pending: bool,
    audio: AudioEngine,
}

impl App {
    pub fn new() -> Self {
        let roots = storage::load_roots();
        let theme = storage::load_theme();
        let playlists = storage::load_playlists()
            .into_iter()
            .map(|playlist| Playlist {
                name: playlist.name,
                track_paths: playlist.tracks,
                tracks: Vec::new(),
                duration: Duration::ZERO,
            })
            .collect();
        let browser_dir = storage::home_dir();
        let browser_entries = library::read_directory_entries(&browser_dir).unwrap_or_default();
        let volume = 70;
        let view = if roots.is_empty() {
            View::Folders
        } else {
            View::Home
        };

        let mut app = Self {
            roots,
            tracks: Vec::new(),
            path_order: Vec::new(),
            recent_indices_cache: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),
            playlists,
            view,
            detail: None,
            selected: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_editing: false,
            current: None,
            state: PlaybackState::Stopped,
            volume,
            shuffle: false,
            repeat: RepeatMode::Off,
            queue_base: Vec::new(),
            queue: Vec::new(),
            queue_pos: None,
            recent_paths: Vec::new(),
            status: String::new(),
            theme,
            help_open: false,
            text_input: None,
            playlist_picker: None,
            browser_dir,
            browser_entries,
            folder_focus: FolderFocus::Browser,
            root_selected: 0,
            browser_selected: 0,
            browser_queue_base: Vec::new(),
            browser_queue: Vec::new(),
            browser_queue_pos: None,
            browser_current: None,
            scan_phase: ScanPhase::Idle,
            scan_rx: None,
            rescan_pending: false,
            audio: AudioEngine::new(volume),
        };

        if !app.roots.is_empty() {
            app.begin_scan();
        }

        app
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q'))
        {
            return true;
        }

        if self.help_open {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
                self.help_open = false;
            }
            return false;
        }

        if self.text_input.is_some() {
            self.handle_text_input_key(key);
            return false;
        }

        if self.playlist_picker.is_some() {
            self.handle_playlist_picker_key(key);
            return false;
        }

        if self.search_editing {
            self.handle_search_key(key);
            return false;
        }

        let result = match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Esc => {
                self.go_back();
                Ok(())
            }
            KeyCode::Char('?') => {
                self.help_open = true;
                Ok(())
            }
            KeyCode::Char('/') if self.view == View::Folders => {
                self.browser_dir = PathBuf::from("/");
                self.folder_focus = FolderFocus::Browser;
                self.refresh_browser();
                Ok(())
            }
            KeyCode::Char('/') => {
                self.open_search();
                Ok(())
            }
            KeyCode::Char('1') => {
                self.set_view(View::Home);
                Ok(())
            }
            KeyCode::Char('2') => {
                self.set_view(View::Search);
                self.search_editing = true;
                Ok(())
            }
            KeyCode::Char('3') => {
                self.set_view(View::Songs);
                Ok(())
            }
            KeyCode::Char('4') => {
                self.set_view(View::Albums);
                Ok(())
            }
            KeyCode::Char('5') => {
                self.set_view(View::Artists);
                Ok(())
            }
            KeyCode::Char('6') => {
                self.set_view(View::Playlists);
                Ok(())
            }
            KeyCode::Char('7') | KeyCode::Char('o') => {
                self.set_view(View::Folders);
                Ok(())
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                Ok(())
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                Ok(())
            }
            KeyCode::PageUp => {
                self.move_selection(-10);
                Ok(())
            }
            KeyCode::PageDown => {
                self.move_selection(10);
                Ok(())
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.select_first();
                Ok(())
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.select_last();
                Ok(())
            }
            KeyCode::Left if self.view == View::Folders => {
                self.folder_focus = FolderFocus::Roots;
                Ok(())
            }
            KeyCode::Right if self.view == View::Folders => {
                self.folder_focus = FolderFocus::Browser;
                Ok(())
            }
            KeyCode::Backspace if self.view == View::Folders => {
                self.browser_up();
                Ok(())
            }
            KeyCode::Enter => self.activate_selected(),
            KeyCode::Char(' ') => self.toggle_playback(),
            KeyCode::Char('n') => self.next_track(false),
            KeyCode::Char('p') => self.previous_track(),
            KeyCode::Char('[') => self.seek(-5),
            KeyCode::Char(']') => self.seek(5),
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.change_volume(5);
                Ok(())
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.change_volume(-5);
                Ok(())
            }
            KeyCode::Char('x') => {
                self.toggle_shuffle();
                Ok(())
            }
            KeyCode::Char('r') => {
                self.cycle_repeat();
                Ok(())
            }
            KeyCode::Char('t') => {
                self.toggle_theme();
                Ok(())
            }
            KeyCode::Char('u') => {
                self.begin_scan();
                Ok(())
            }
            KeyCode::Char('c') if self.view == View::Playlists => {
                self.open_create_playlist(Vec::new());
                Ok(())
            }
            KeyCode::Char('e') if self.view == View::Playlists => self.open_rename_playlist(),
            KeyCode::Char('a') if self.view == View::Folders => self.add_browser_root(),
            KeyCode::Char('a') => self.open_playlist_picker(),
            KeyCode::Char('d') if self.view == View::Folders => self.remove_selected_root(),
            KeyCode::Char('d') if self.view == View::Playlists => self.remove_selected_playlist_track(),
            KeyCode::Char('D') if self.view == View::Playlists => self.delete_selected_playlist(),
            KeyCode::Char('~') if self.view == View::Folders => {
                self.browser_dir = storage::home_dir();
                self.refresh_browser();
                Ok(())
            }
            _ => Ok(()),
        };

        if let Err(error) = result {
            self.status = error.to_string();
        }

        false
    }

    pub fn tick(&mut self) -> bool {
        let mut changed = self.poll_scan();

        if self.state == PlaybackState::Playing && self.audio.is_empty() {
            changed = true;
            if let Err(error) = self.next_track(true) {
                self.status = error.to_string();
                self.state = PlaybackState::Stopped;
            }
        }

        changed
    }

    pub fn poll_interval(&self) -> Duration {
        if self.scan_rx.is_some() {
            Duration::from_millis(50)
        } else if self.state == PlaybackState::Playing {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(30)
        }
    }

    pub fn progress_epoch(&self) -> u64 {
        if self.state == PlaybackState::Playing {
            (self.audio.position().as_millis() / 500) as u64
        } else {
            0
        }
    }

    fn handle_text_input_key(&mut self, key: KeyEvent) {
        let Some(mut input) = self.text_input.take() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {}
            KeyCode::Enter => {
                if let Err(error) = self.commit_text_input(input) {
                    self.status = error.to_string();
                }
            }
            KeyCode::Backspace => {
                input.value.pop();
                self.text_input = Some(input);
            }
            KeyCode::Delete => {
                input.value.clear();
                self.text_input = Some(input);
            }
            KeyCode::Char(character)
                if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && input.value.chars().count() < 80 =>
            {
                input.value.push(character);
                self.text_input = Some(input);
            }
            _ => self.text_input = Some(input),
        }
    }

    fn handle_playlist_picker_key(&mut self, key: KeyEvent) {
        let Some(mut picker) = self.playlist_picker.take() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {}
            KeyCode::Up | KeyCode::Char('k') => {
                picker.selected = move_index(picker.selected, self.playlists.len(), -1);
                self.playlist_picker = Some(picker);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                picker.selected = move_index(picker.selected, self.playlists.len(), 1);
                self.playlist_picker = Some(picker);
            }
            KeyCode::PageUp => {
                picker.selected = move_index(picker.selected, self.playlists.len(), -10);
                self.playlist_picker = Some(picker);
            }
            KeyCode::PageDown => {
                picker.selected = move_index(picker.selected, self.playlists.len(), 10);
                self.playlist_picker = Some(picker);
            }
            KeyCode::Char('c') => self.open_create_playlist(picker.track_paths),
            KeyCode::Enter => {
                if let Err(error) = self.add_paths_to_playlist(picker.selected, picker.track_paths) {
                    self.status = error.to_string();
                }
            }
            _ => self.playlist_picker = Some(picker),
        }
    }

    fn open_create_playlist(&mut self, pending_paths: Vec<PathBuf>) {
        self.text_input = Some(TextInput {
            prompt: if pending_paths.is_empty() {
                "Create playlist".to_owned()
            } else {
                "Create playlist and add selection".to_owned()
            },
            value: String::new(),
            kind: TextInputKind::CreatePlaylist,
            pending_paths,
        });
    }

    fn open_rename_playlist(&mut self) -> Result<()> {
        let Some(index) = self.selected_playlist_index() else {
            self.status = "Select a playlist first".to_owned();
            return Ok(());
        };
        let Some(playlist) = self.playlists.get(index) else {
            return Ok(());
        };

        self.text_input = Some(TextInput {
            prompt: "Rename playlist".to_owned(),
            value: playlist.name.clone(),
            kind: TextInputKind::RenamePlaylist(index),
            pending_paths: Vec::new(),
        });
        Ok(())
    }

    fn commit_text_input(&mut self, input: TextInput) -> Result<()> {
        let name = input.value.trim().to_owned();
        if name.is_empty() {
            self.status = "Playlist name cannot be empty".to_owned();
            self.text_input = Some(input);
            return Ok(());
        }

        let ignored = match input.kind {
            TextInputKind::RenamePlaylist(index) => Some(index),
            TextInputKind::CreatePlaylist => None,
        };
        if self.playlists.iter().enumerate().any(|(index, playlist)| {
            Some(index) != ignored && playlist.name.eq_ignore_ascii_case(&name)
        }) {
            self.status = format!("Playlist '{name}' already exists");
            self.text_input = Some(input);
            return Ok(());
        }

        match input.kind {
            TextInputKind::CreatePlaylist => {
                let mut track_paths = Vec::new();
                for path in input.pending_paths {
                    if !track_paths.iter().any(|existing| existing == &path) {
                        track_paths.push(path);
                    }
                }
                self.playlists.push(Playlist {
                    name: name.clone(),
                    track_paths,
                    tracks: Vec::new(),
                    duration: Duration::ZERO,
                });
                self.rebuild_playlist_indexes();
                self.save_playlists()?;
                self.rebuild_search();
                self.view = View::Playlists;
                self.detail = None;
                self.selected = self.playlists.len().saturating_sub(1);
                let count = self.playlists[self.selected].track_paths.len();
                self.status = if count == 0 {
                    format!("Created playlist '{name}'")
                } else {
                    format!("Created playlist '{name}' with {count} songs")
                };
            }
            TextInputKind::RenamePlaylist(index) => {
                if let Some(playlist) = self.playlists.get_mut(index) {
                    playlist.name = name.clone();
                } else {
                    return Ok(());
                }
                self.save_playlists()?;
                self.rebuild_search();
                self.status = format!("Renamed playlist to '{name}'");
            }
        }

        Ok(())
    }

    fn open_playlist_picker(&mut self) -> Result<()> {
        let Some((indices, source_label)) = self.selected_tracks_for_playlist() else {
            self.status = "Select a song, album, artist, or playlist first".to_owned();
            return Ok(());
        };

        let mut track_paths = Vec::new();
        for index in indices {
            if let Some(track) = self.tracks.get(index) {
                if !track_paths.iter().any(|path| path == &track.path) {
                    track_paths.push(track.path.clone());
                }
            }
        }
        if track_paths.is_empty() {
            self.status = "The selection contains no available songs".to_owned();
            return Ok(());
        }

        if self.playlists.is_empty() {
            self.open_create_playlist(track_paths);
        } else {
            self.playlist_picker = Some(PlaylistPicker {
                selected: 0,
                track_paths,
                source_label,
            });
        }
        Ok(())
    }

    fn selected_tracks_for_playlist(&self) -> Option<(Vec<usize>, String)> {
        match self.view {
            View::Home => {
                let recent = self.recent_indices();
                let track = recent.get(self.selected).copied()?;
                Some((vec![track], self.tracks.get(track)?.title.to_string()))
            }
            View::Search => match *self.search_results.get(self.selected)? {
                SearchItem::Playlist(index) => {
                    let playlist = self.playlists.get(index)?;
                    Some((playlist.tracks.clone(), playlist.name.clone()))
                }
                SearchItem::Artist(index) => {
                    let artist = self.artists.get(index)?;
                    Some((artist.tracks.clone(), artist.name.to_string()))
                }
                SearchItem::Album(index) => {
                    let album = self.albums.get(index)?;
                    Some((album.tracks.clone(), album.title.to_string()))
                }
                SearchItem::Track(index) => {
                    Some((vec![index], self.tracks.get(index)?.title.to_string()))
                }
            },
            View::Songs => {
                let track = self.selected;
                Some((vec![track], self.tracks.get(track)?.title.to_string()))
            }
            View::Albums => {
                let album_index = match self.detail {
                    Some(DetailView::Album(index)) => index,
                    _ => self.selected,
                };
                let album = self.albums.get(album_index)?;
                if matches!(self.detail, Some(DetailView::Album(_))) {
                    let track = *album.tracks.get(self.selected)?;
                    Some((vec![track], self.tracks.get(track)?.title.to_string()))
                } else {
                    Some((album.tracks.clone(), album.title.to_string()))
                }
            }
            View::Artists => {
                let artist_index = match self.detail {
                    Some(DetailView::Artist(index)) => index,
                    _ => self.selected,
                };
                let artist = self.artists.get(artist_index)?;
                if matches!(self.detail, Some(DetailView::Artist(_))) {
                    let track = *artist.tracks.get(self.selected)?;
                    Some((vec![track], self.tracks.get(track)?.title.to_string()))
                } else {
                    Some((artist.tracks.clone(), artist.name.to_string()))
                }
            }
            View::Playlists => {
                let playlist_index = self.selected_playlist_index()?;
                let playlist = self.playlists.get(playlist_index)?;
                if matches!(self.detail, Some(DetailView::Playlist(_))) {
                    let track = *playlist.tracks.get(self.selected)?;
                    Some((vec![track], self.tracks.get(track)?.title.to_string()))
                } else {
                    Some((playlist.tracks.clone(), playlist.name.clone()))
                }
            }
            View::Folders => None,
        }
    }

    fn add_paths_to_playlist(&mut self, index: usize, paths: Vec<PathBuf>) -> Result<()> {
        let (name, added) = {
            let Some(playlist) = self.playlists.get_mut(index) else {
                return Ok(());
            };
            let name = playlist.name.clone();
            let mut added = 0;
            for path in paths {
                if !playlist.track_paths.iter().any(|existing| existing == &path) {
                    playlist.track_paths.push(path);
                    added += 1;
                }
            }
            (name, added)
        };

        self.rebuild_playlist_indexes();
        self.save_playlists()?;
        self.status = if added == 0 {
            format!("All selected songs are already in '{name}'")
        } else {
            format!("Added {added} songs to '{name}'")
        };
        Ok(())
    }

    fn remove_selected_playlist_track(&mut self) -> Result<()> {
        let Some(DetailView::Playlist(playlist_index)) = self.detail else {
            self.status = "Open a playlist to remove one of its songs".to_owned();
            return Ok(());
        };
        let Some(track_index) = self
            .playlists
            .get(playlist_index)
            .and_then(|playlist| playlist.tracks.get(self.selected))
            .copied()
        else {
            return Ok(());
        };
        let Some(path) = self.tracks.get(track_index).map(|track| track.path.clone()) else {
            return Ok(());
        };

        if let Some(playlist) = self.playlists.get_mut(playlist_index) {
            playlist.track_paths.retain(|existing| existing != &path);
        }
        self.rebuild_playlist_indexes();
        self.save_playlists()?;
        self.selected = self.selected.min(self.selection_len().saturating_sub(1));
        self.status = "Removed song from playlist".to_owned();
        Ok(())
    }

    fn delete_selected_playlist(&mut self) -> Result<()> {
        let Some(index) = self.selected_playlist_index() else {
            return Ok(());
        };
        let name = self.playlists[index].name.clone();
        self.playlists.remove(index);
        self.detail = None;
        let next_selected = index.min(self.playlists.len().saturating_sub(1));
        self.save_playlists()?;
        self.rebuild_search();
        self.selected = next_selected;
        self.status = format!("Deleted playlist '{name}'");
        Ok(())
    }

    fn selected_playlist_index(&self) -> Option<usize> {
        match self.detail {
            Some(DetailView::Playlist(index)) => self.playlists.get(index).map(|_| index),
            _ if self.view == View::Playlists => self.playlists.get(self.selected).map(|_| self.selected),
            _ => None,
        }
    }

    fn save_playlists(&self) -> Result<()> {
        storage::save_playlists(
            self.playlists
                .iter()
                .map(|playlist| (playlist.name.as_str(), playlist.track_paths.as_slice())),
        )
    }

    fn rebuild_playlist_indexes(&mut self) {
        let tracks = &self.tracks;
        let path_order = &self.path_order;

        for playlist in &mut self.playlists {
            playlist.tracks.clear();
            playlist.tracks.reserve(playlist.track_paths.len());
            for path in &playlist.track_paths {
                if let Ok(position) = path_order.binary_search_by(|index| {
                    tracks[*index].path.as_path().cmp(path.as_path())
                }) {
                    playlist.tracks.push(path_order[position]);
                }
            }
            playlist.duration = playlist.tracks.iter().fold(Duration::ZERO, |total, index| {
                total.saturating_add(tracks[*index].duration.unwrap_or_default())
            });
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.search_editing = false,
            KeyCode::Enter => {
                self.search_editing = false;
                if let Err(error) = self.activate_selected() {
                    self.status = error.to_string();
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.rebuild_search();
            }
            KeyCode::Delete => {
                self.search_query.clear();
                self.rebuild_search();
            }
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::Char(character)
                if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.search_query.push(character);
                self.rebuild_search();
            }
            _ => {}
        }
    }

    fn set_view(&mut self, view: View) {
        self.view = view;
        self.detail = None;
        self.selected = 0;
        if view != View::Search {
            self.search_editing = false;
        }
    }

    fn open_search(&mut self) {
        self.set_view(View::Search);
        self.search_editing = true;
    }

    fn go_back(&mut self) {
        if self.detail.take().is_some() {
            self.selected = 0;
            return;
        }
        if self.view == View::Search && !self.search_query.is_empty() {
            self.search_query.clear();
            self.rebuild_search();
            return;
        }
        if self.view != View::Home && !self.tracks.is_empty() {
            self.set_view(View::Home);
        }
    }

    fn poll_scan(&mut self) -> bool {
        let Some(receiver) = self.scan_rx.take() else {
            return false;
        };

        let mut changed = false;
        let mut keep_receiver = true;
        let mut disconnected = false;

        loop {
            match receiver.try_recv() {
                Ok(ScanEvent::Progress { done, total }) => {
                    changed = true;
                    self.scan_phase = ScanPhase::Reading { done, total };
                    self.status = format!("Reading metadata: {done}/{total}");
                }
                Ok(ScanEvent::Finished(result)) => {
                    changed = true;
                    keep_receiver = false;
                    self.scan_phase = ScanPhase::Idle;
                    match result {
                        Ok(tracks) if !self.roots.is_empty() => self.apply_scan_result(tracks),
                        Ok(_) => self.status = "All library folders removed".to_owned(),
                        Err(error) => self.status = format!("Library scan failed: {error}"),
                    }
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    changed = true;
                    keep_receiver = false;
                    disconnected = true;
                    break;
                }
            }
        }

        if keep_receiver {
            self.scan_rx = Some(receiver);
        } else {
            if disconnected {
                self.scan_phase = ScanPhase::Idle;
                self.status = "Library scan stopped unexpectedly".to_owned();
            }
            if self.rescan_pending {
                self.rescan_pending = false;
                self.begin_scan();
            }
        }

        changed
    }

    fn begin_scan(&mut self) {
        if self.roots.is_empty() {
            self.status = "Add at least one library folder first".to_owned();
            return;
        }
        if self.scan_rx.is_some() {
            self.rescan_pending = true;
            self.status = "A fresh rescan is queued".to_owned();
            return;
        }

        self.scan_phase = ScanPhase::Discovering;
        self.status = "Discovering audio files...".to_owned();
        self.scan_rx = Some(library::spawn_scan(self.roots.clone()));
    }

    fn apply_scan_result(&mut self, tracks: Vec<Track>) {
        let old_current_path = self.current_track().map(|track| track.path.clone());
        let old_queue_paths: Vec<PathBuf> = self
            .queue_base
            .iter()
            .filter_map(|index| self.tracks.get(*index))
            .map(|track| track.path.clone())
            .collect();

        self.tracks = tracks;
        self.rebuild_indexes();

        let restored_current = old_current_path
            .as_deref()
            .and_then(|path| find_track_index(&self.tracks, &self.path_order, path));
        self.current = restored_current;
        if self.current.is_some() {
            self.browser_current = None;
            self.browser_queue_base.clear();
            self.browser_queue.clear();
            self.browser_queue_pos = None;
        }

        self.queue_base = old_queue_paths
            .iter()
            .filter_map(|path| find_track_index(&self.tracks, &self.path_order, path))
            .collect();

        if self.queue_base.is_empty() {
            if let Some(current) = self.current {
                self.queue_base.push(current);
            }
        }
        self.rebuild_queue_order();
        self.refresh_recent_indices();

        if old_current_path.is_some() && self.current.is_none() {
            self.stop();
        }

        self.selected = 0;
        self.root_selected = self.root_selected.min(self.roots.len().saturating_sub(1));
        self.status = format!(
            "Library ready: {} songs, {} albums, {} artists",
            self.tracks.len(),
            self.albums.len(),
            self.artists.len()
        );

        if self.tracks.is_empty() {
            self.status = "No supported audio files found in the selected folders".to_owned();
        }
    }

    fn rebuild_indexes(&mut self) {
        self.path_order.clear();
        self.path_order.extend(0..self.tracks.len());
        let tracks = &self.tracks;
        self.path_order.sort_unstable_by(|left, right| {
            tracks[*left].path.cmp(&tracks[*right].path)
        });

        let mut album_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut artist_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();

        for (index, track) in self.tracks.iter().enumerate() {
            let mut album_key = track.album_dir.to_string_lossy().to_lowercase();
            album_key.push('\0');
            album_key.push_str(&track.album.to_lowercase());
            album_map.entry(album_key).or_default().push(index);
            artist_map
                .entry(track.artist.to_lowercase())
                .or_default()
                .push(index);
        }

        let mut albums: Vec<Album> = album_map
            .into_values()
            .map(|mut tracks| {
                tracks.sort_by(|left, right| {
                    compare_album_tracks(&self.tracks[*left], &self.tracks[*right])
                });
                let first = &self.tracks[tracks[0]];
                let mut album_artist: Option<Arc<str>> = None;
                let mut various_artists = false;
                for index in &tracks {
                    let candidate = &self.tracks[*index].album_artist;
                    if candidate.trim().is_empty() {
                        continue;
                    }
                    match &album_artist {
                        None => album_artist = Some(Arc::clone(candidate)),
                        Some(existing) if existing.as_ref() == candidate.as_ref() => {}
                        Some(_) => {
                            various_artists = true;
                            break;
                        }
                    }
                }
                let artist = if various_artists {
                    Arc::from("Various Artists")
                } else {
                    album_artist.unwrap_or_else(|| Arc::clone(&first.artist))
                };
                let duration = tracks.iter().fold(Duration::ZERO, |total, index| {
                    total.saturating_add(self.tracks[*index].duration.unwrap_or_default())
                });
                Album {
                    title: Arc::clone(&first.album),
                    artist,
                    tracks,
                    duration,
                }
            })
            .collect();

        albums.sort_by_cached_key(|album| {
            (album.artist.to_lowercase(), album.title.to_lowercase())
        });

        let mut artists: Vec<Artist> = artist_map
            .into_values()
            .map(|mut tracks| {
                tracks.sort_by(|left, right| {
                    compare_artist_tracks(&self.tracks[*left], &self.tracks[*right])
                });
                let name = Arc::clone(&self.tracks[tracks[0]].artist);
                let album_count = tracks
                    .iter()
                    .map(|index| {
                        (
                            self.tracks[*index].album_dir.as_ref(),
                            self.tracks[*index].album.as_ref(),
                        )
                    })
                    .collect::<BTreeSet<_>>()
                    .len();
                Artist {
                    name,
                    tracks,
                    album_count,
                }
            })
            .collect();

        artists.sort_by_cached_key(|artist| artist.name.to_lowercase());
        self.albums = albums;
        self.artists = artists;
        self.rebuild_playlist_indexes();
        self.rebuild_search();
    }

    fn rebuild_search(&mut self) {
        self.search_results.clear();
        self.selected = 0;

        let tokens: Vec<String> = self
            .search_query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        if tokens.is_empty() {
            return;
        }

        for (index, playlist) in self.playlists.iter().enumerate() {
            if matches_fields(&[playlist.name.as_str()], &tokens) {
                self.search_results.push(SearchItem::Playlist(index));
            }
        }
        for (index, artist) in self.artists.iter().enumerate() {
            if matches_fields(&[artist.name.as_ref()], &tokens) {
                self.search_results.push(SearchItem::Artist(index));
            }
        }
        for (index, album) in self.albums.iter().enumerate() {
            if matches_fields(&[album.title.as_ref(), album.artist.as_ref()], &tokens) {
                self.search_results.push(SearchItem::Album(index));
            }
        }
        for (index, track) in self.tracks.iter().enumerate() {
            let path = track.path.to_string_lossy();
            if matches_fields(
                &[
                    track.title.as_ref(),
                    track.artist.as_ref(),
                    track.album_artist.as_ref(),
                    track.album.as_ref(),
                    path.as_ref(),
                ],
                &tokens,
            ) {
                self.search_results.push(SearchItem::Track(index));
            }
        }
    }

    fn activate_selected(&mut self) -> Result<()> {
        match self.view {
            View::Home => {
                let recent = self.recent_indices();
                if let Some(track) = recent.get(self.selected).copied() {
                    let queue = recent.to_vec();
                    self.play_queue(queue, track)?;
                }
            }
            View::Search => {
                let Some(item) = self.search_results.get(self.selected).copied() else {
                    return Ok(());
                };
                match item {
                    SearchItem::Playlist(index) => {
                        self.view = View::Playlists;
                        self.detail = Some(DetailView::Playlist(index));
                        self.selected = 0;
                    }
                    SearchItem::Artist(index) => {
                        self.view = View::Artists;
                        self.detail = Some(DetailView::Artist(index));
                        self.selected = 0;
                    }
                    SearchItem::Album(index) => {
                        self.view = View::Albums;
                        self.detail = Some(DetailView::Album(index));
                        self.selected = 0;
                    }
                    SearchItem::Track(track) => {
                        let queue: Vec<usize> = self
                            .search_results
                            .iter()
                            .filter_map(|item| match item {
                                SearchItem::Track(index) => Some(*index),
                                _ => None,
                            })
                            .collect();
                        self.play_queue(queue, track)?;
                    }
                }
            }
            View::Songs => {
                if self.selected < self.tracks.len() {
                    let queue: Vec<usize> = (0..self.tracks.len()).collect();
                    self.play_queue(queue, self.selected)?;
                }
            }
            View::Albums => match self.detail {
                Some(DetailView::Album(album_index)) => {
                    if let Some(album) = self.albums.get(album_index) {
                        let queue = album.tracks.clone();
                        if let Some(track) = queue.get(self.selected).copied() {
                            self.play_queue(queue, track)?;
                        }
                    }
                }
                _ => {
                    if self.selected < self.albums.len() {
                        self.detail = Some(DetailView::Album(self.selected));
                        self.selected = 0;
                    }
                }
            },
            View::Artists => match self.detail {
                Some(DetailView::Artist(artist_index)) => {
                    if let Some(artist) = self.artists.get(artist_index) {
                        let queue = artist.tracks.clone();
                        if let Some(track) = queue.get(self.selected).copied() {
                            self.play_queue(queue, track)?;
                        }
                    }
                }
                _ => {
                    if self.selected < self.artists.len() {
                        self.detail = Some(DetailView::Artist(self.selected));
                        self.selected = 0;
                    }
                }
            },
            View::Playlists => match self.detail {
                Some(DetailView::Playlist(playlist_index)) => {
                    if let Some(playlist) = self.playlists.get(playlist_index) {
                        let queue = playlist.tracks.clone();
                        if let Some(track) = queue.get(self.selected).copied() {
                            self.play_queue(queue, track)?;
                        }
                    }
                }
                _ => {
                    if self.selected < self.playlists.len() {
                        self.detail = Some(DetailView::Playlist(self.selected));
                        self.selected = 0;
                    }
                }
            },
            View::Folders => self.activate_folder()?,
        }
        Ok(())
    }

    fn play_queue(&mut self, queue: Vec<usize>, track: usize) -> Result<()> {
        if queue.is_empty() || self.tracks.get(track).is_none() {
            return Ok(());
        }
        self.browser_queue_base.clear();
        self.browser_queue.clear();
        self.browser_queue_pos = None;
        self.browser_current = None;
        self.queue_base = queue;
        self.current = Some(track);
        self.rebuild_queue_order();
        self.play_track(track)
    }

    fn play_track(&mut self, index: usize) -> Result<()> {
        let Some(track) = self.tracks.get(index) else {
            return Ok(());
        };
        let path = track.path.clone();
        let title = track.title.clone();

        self.audio.play_file(&path)?;
        self.browser_current = None;
        self.current = Some(index);
        self.queue_pos = self.queue.iter().position(|queued| *queued == index);
        self.state = PlaybackState::Playing;
        self.status = format!("Playing {title}");

        self.recent_paths.retain(|recent| recent != &path);
        self.recent_paths.insert(0, path);
        self.recent_paths.truncate(50);
        self.recent_indices_cache.retain(|recent| *recent != index);
        self.recent_indices_cache.insert(0, index);
        self.recent_indices_cache.truncate(50);
        Ok(())
    }

    fn play_browser_queue(&mut self, queue: Vec<PathBuf>, selected_path: &Path) -> Result<()> {
        if queue.is_empty() {
            return Ok(());
        }

        self.queue_base.clear();
        self.queue.clear();
        self.queue_pos = None;
        self.current = None;
        self.browser_queue_base = queue;
        self.browser_current = None;
        self.rebuild_browser_queue_order();

        let position = self
            .browser_queue
            .iter()
            .position(|path| path.as_path() == selected_path)
            .unwrap_or(0);
        self.play_browser_at(position)
    }

    fn play_browser_at(&mut self, position: usize) -> Result<()> {
        let Some(path) = self.browser_queue.get(position).cloned() else {
            return Ok(());
        };
        let track = Track::from_path(path.clone());

        self.audio.play_file(&path)?;
        self.current = None;
        self.browser_queue_pos = Some(position);
        self.browser_current = Some(track);
        self.state = PlaybackState::Playing;
        self.status = format!(
            "Playing {}",
            self.browser_current.as_ref().map_or("", |track| track.title.as_ref())
        );

        self.recent_paths.retain(|recent| recent != &path);
        self.recent_paths.insert(0, path);
        self.recent_paths.truncate(50);
        self.refresh_recent_indices();
        Ok(())
    }

    fn toggle_playback(&mut self) -> Result<()> {
        match self.state {
            PlaybackState::Playing => {
                self.audio.pause();
                self.state = PlaybackState::Paused;
                self.status = "Paused".to_owned();
            }
            PlaybackState::Paused => {
                self.audio.resume();
                self.state = PlaybackState::Playing;
                self.status = "Playing".to_owned();
            }
            PlaybackState::Stopped => {
                if self.browser_current.is_some() {
                    let position = self.browser_queue_pos.unwrap_or(0);
                    self.play_browser_at(position)?;
                } else if let Some(current) = self.current {
                    if self.queue_base.is_empty() {
                        self.queue_base.push(current);
                        self.rebuild_queue_order();
                    }
                    self.play_track(current)?;
                } else if let Some((queue, track)) = self.queue_for_selection() {
                    self.play_queue(queue, track)?;
                } else {
                    self.status = "Select a song, album, artist, or playlist first".to_owned();
                }
            }
        }
        Ok(())
    }

    fn queue_for_selection(&self) -> Option<(Vec<usize>, usize)> {
        match self.view {
            View::Home => {
                let queue = self.recent_indices();
                let track = queue.get(self.selected).copied()?;
                Some((queue.to_vec(), track))
            }
            View::Search => {
                let item = self.search_results.get(self.selected)?;
                match *item {
                    SearchItem::Playlist(index) => {
                        let queue = self.playlists.get(index)?.tracks.clone();
                        let track = queue.first().copied()?;
                        Some((queue, track))
                    }
                    SearchItem::Track(track) => {
                        let queue: Vec<usize> = self
                            .search_results
                            .iter()
                            .filter_map(|item| match item {
                                SearchItem::Track(index) => Some(*index),
                                _ => None,
                            })
                            .collect();
                        Some((queue, track))
                    }
                    SearchItem::Album(index) => {
                        let queue = self.albums.get(index)?.tracks.clone();
                        let track = queue.first().copied()?;
                        Some((queue, track))
                    }
                    SearchItem::Artist(index) => {
                        let queue = self.artists.get(index)?.tracks.clone();
                        let track = queue.first().copied()?;
                        Some((queue, track))
                    }
                }
            }
            View::Songs => {
                let queue: Vec<usize> = (0..self.tracks.len()).collect();
                let track = queue.get(self.selected).copied()?;
                Some((queue, track))
            }
            View::Albums => {
                let album_index = match self.detail {
                    Some(DetailView::Album(index)) => index,
                    _ => self.selected,
                };
                let queue = self.albums.get(album_index)?.tracks.clone();
                let track = if matches!(self.detail, Some(DetailView::Album(_))) {
                    queue.get(self.selected).copied()
                } else {
                    queue.first().copied()
                }?;
                Some((queue, track))
            }
            View::Artists => {
                let artist_index = match self.detail {
                    Some(DetailView::Artist(index)) => index,
                    _ => self.selected,
                };
                let queue = self.artists.get(artist_index)?.tracks.clone();
                let track = if matches!(self.detail, Some(DetailView::Artist(_))) {
                    queue.get(self.selected).copied()
                } else {
                    queue.first().copied()
                }?;
                Some((queue, track))
            }
            View::Playlists => {
                let playlist_index = match self.detail {
                    Some(DetailView::Playlist(index)) => index,
                    _ => self.selected,
                };
                let queue = self.playlists.get(playlist_index)?.tracks.clone();
                let track = if matches!(self.detail, Some(DetailView::Playlist(_))) {
                    queue.get(self.selected).copied()
                } else {
                    queue.first().copied()
                }?;
                Some((queue, track))
            }
            View::Folders => None,
        }
    }

    fn next_track(&mut self, automatic: bool) -> Result<()> {
        if self.browser_current.is_some() {
            if automatic && self.repeat == RepeatMode::One {
                return self.play_browser_at(self.browser_queue_pos.unwrap_or(0));
            }

            if self.browser_queue.is_empty() {
                return Ok(());
            }
            let current_pos = self.browser_queue_pos.unwrap_or(0);
            let next_pos = if current_pos + 1 < self.browser_queue.len() {
                current_pos + 1
            } else if self.repeat == RepeatMode::All {
                0
            } else {
                self.stop();
                self.status = "Folder queue finished".to_owned();
                return Ok(());
            };
            return self.play_browser_at(next_pos);
        }

        if automatic && self.repeat == RepeatMode::One {
            if let Some(current) = self.current {
                return self.play_track(current);
            }
        }

        if self.queue.is_empty() {
            if self.tracks.is_empty() {
                return Ok(());
            }
            self.queue_base = (0..self.tracks.len()).collect();
            self.rebuild_queue_order();
        }

        let current_pos = self
            .current
            .and_then(|current| self.queue.iter().position(|index| *index == current))
            .or(self.queue_pos)
            .unwrap_or(0);

        let next_pos = if current_pos + 1 < self.queue.len() {
            current_pos + 1
        } else if self.repeat == RepeatMode::All {
            0
        } else {
            self.stop();
            self.status = "Queue finished".to_owned();
            return Ok(());
        };

        self.queue_pos = Some(next_pos);
        let next_track = self.queue[next_pos];
        self.play_track(next_track)
    }

    fn previous_track(&mut self) -> Result<()> {
        if self.audio.position().as_secs_f64() > 3.0 {
            self.audio.seek_to(0.0)?;
            self.status = "Restarted current song".to_owned();
            return Ok(());
        }

        if self.browser_current.is_some() {
            if self.browser_queue.is_empty() {
                return Ok(());
            }
            let current_pos = self.browser_queue_pos.unwrap_or(0);
            let previous_pos = if current_pos > 0 {
                current_pos - 1
            } else if self.repeat == RepeatMode::All {
                self.browser_queue.len() - 1
            } else {
                0
            };
            return self.play_browser_at(previous_pos);
        }

        if self.queue.is_empty() {
            return Ok(());
        }

        let current_pos = self
            .current
            .and_then(|current| self.queue.iter().position(|index| *index == current))
            .or(self.queue_pos)
            .unwrap_or(0);
        let previous_pos = if current_pos > 0 {
            current_pos - 1
        } else if self.repeat == RepeatMode::All {
            self.queue.len() - 1
        } else {
            0
        };

        self.queue_pos = Some(previous_pos);
        let previous_track = self.queue[previous_pos];
        self.play_track(previous_track)
    }

    fn stop(&mut self) {
        self.audio.stop();
        self.state = PlaybackState::Stopped;
    }

    fn seek(&mut self, seconds: i64) -> Result<()> {
        if self.state == PlaybackState::Stopped {
            self.status = "Nothing is playing".to_owned();
            return Ok(());
        }
        self.audio.seek_by(seconds)?;
        self.status = format!("Seek {seconds:+} seconds");
        Ok(())
    }

    fn change_volume(&mut self, delta: i16) {
        self.volume = (self.volume as i16 + delta).clamp(0, 100) as u8;
        self.audio.set_volume(self.volume);
        self.status = format!("Volume {}%", self.volume);
    }

    fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.rebuild_queue_order();
        self.rebuild_browser_queue_order();
        self.status = format!("Shuffle {}", if self.shuffle { "on" } else { "off" });
    }

    fn cycle_repeat(&mut self) {
        self.repeat = match self.repeat {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        self.status = format!("Repeat {}", self.repeat.label());
    }

    fn rebuild_queue_order(&mut self) {
        self.queue.clone_from(&self.queue_base);
        let current = self.current;

        if self.shuffle {
            shuffle_slice(&mut self.queue);
            if let Some(current) = current {
                if let Some(position) = self.queue.iter().position(|index| *index == current) {
                    self.queue.swap(0, position);
                }
            }
        }

        self.queue_pos = current.and_then(|current| self.queue.iter().position(|index| *index == current));
    }

    fn rebuild_browser_queue_order(&mut self) {
        let current_path = self.browser_current.as_ref().map(|track| track.path.clone());
        self.browser_queue.clone_from(&self.browser_queue_base);

        if self.shuffle {
            shuffle_slice(&mut self.browser_queue);
            if let Some(path) = current_path.as_ref() {
                if let Some(position) = self.browser_queue.iter().position(|queued| queued == path) {
                    self.browser_queue.swap(0, position);
                }
            }
        }

        self.browser_queue_pos = current_path
            .as_ref()
            .and_then(|path| self.browser_queue.iter().position(|queued| queued == path));
    }

    fn toggle_theme(&mut self) {
        self.theme = match self.theme {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
        if let Err(error) = storage::save_theme(self.theme) {
            self.status = format!("Theme changed, but could not be saved: {error}");
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.view == View::Folders {
            match self.folder_focus {
                FolderFocus::Roots => {
                    self.root_selected = move_index(self.root_selected, self.roots.len(), delta)
                }
                FolderFocus::Browser => {
                    self.browser_selected =
                        move_index(self.browser_selected, self.browser_entries.len(), delta)
                }
            }
            return;
        }

        self.selected = move_index(self.selected, self.selection_len(), delta);
    }

    fn select_first(&mut self) {
        if self.view == View::Folders {
            match self.folder_focus {
                FolderFocus::Roots => self.root_selected = 0,
                FolderFocus::Browser => self.browser_selected = 0,
            }
        } else {
            self.selected = 0;
        }
    }

    fn select_last(&mut self) {
        if self.view == View::Folders {
            match self.folder_focus {
                FolderFocus::Roots => self.root_selected = self.roots.len().saturating_sub(1),
                FolderFocus::Browser => {
                    self.browser_selected = self.browser_entries.len().saturating_sub(1)
                }
            }
        } else {
            self.selected = self.selection_len().saturating_sub(1);
        }
    }

    pub fn selection_len(&self) -> usize {
        match self.view {
            View::Home => self.recent_indices().len(),
            View::Search => self.search_results.len(),
            View::Songs => self.tracks.len(),
            View::Albums => match self.detail {
                Some(DetailView::Album(index)) => self
                    .albums
                    .get(index)
                    .map_or(0, |album| album.tracks.len()),
                _ => self.albums.len(),
            },
            View::Artists => match self.detail {
                Some(DetailView::Artist(index)) => self
                    .artists
                    .get(index)
                    .map_or(0, |artist| artist.tracks.len()),
                _ => self.artists.len(),
            },
            View::Playlists => match self.detail {
                Some(DetailView::Playlist(index)) => self
                    .playlists
                    .get(index)
                    .map_or(0, |playlist| playlist.tracks.len()),
                _ => self.playlists.len(),
            },
            View::Folders => match self.folder_focus {
                FolderFocus::Roots => self.roots.len(),
                FolderFocus::Browser => self.browser_entries.len(),
            },
        }
    }

    fn activate_folder(&mut self) -> Result<()> {
        match self.folder_focus {
            FolderFocus::Roots => {
                if let Some(root) = self.roots.get(self.root_selected).cloned() {
                    self.browser_dir = root;
                    self.folder_focus = FolderFocus::Browser;
                    self.refresh_browser();
                }
            }
            FolderFocus::Browser => {
                if let Some(entry) = self.browser_entries.get(self.browser_selected).cloned() {
                    match entry.kind {
                        DirectoryEntryKind::Directory(path) => {
                            self.browser_dir = path;
                            self.refresh_browser();
                        }
                        DirectoryEntryKind::Track(track) => {
                            let queue: Vec<PathBuf> = self
                                .browser_entries
                                .iter()
                                .filter_map(|entry| match &entry.kind {
                                    DirectoryEntryKind::Track(track) => Some(track.path.clone()),
                                    DirectoryEntryKind::Directory(_) => None,
                                })
                                .collect();
                            self.play_browser_queue(queue, &track.path)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn browser_up(&mut self) {
        if let Some(parent) = self.browser_dir.parent().map(PathBuf::from) {
            self.browser_dir = parent;
            self.refresh_browser();
        }
    }

    fn refresh_browser(&mut self) {
        match library::read_directory_entries(&self.browser_dir) {
            Ok(entries) => {
                self.browser_entries = entries;
                self.browser_selected = 0;
                self.status = self.browser_dir.display().to_string();
            }
            Err(error) => {
                self.browser_entries.clear();
                self.browser_selected = 0;
                self.status = format!("Cannot read folder: {error}");
            }
        }
    }

    fn add_browser_root(&mut self) -> Result<()> {
        let root = self
            .browser_dir
            .canonicalize()
            .unwrap_or_else(|_| self.browser_dir.clone());
        if self.roots.iter().any(|existing| existing == &root) {
            self.status = "This folder is already in the library".to_owned();
            return Ok(());
        }

        self.roots.push(root);
        self.roots.sort();
        storage::save_roots(&self.roots)?;
        self.root_selected = self.roots.len().saturating_sub(1);
        self.status = "Library folder added".to_owned();
        self.begin_scan();
        Ok(())
    }

    fn remove_selected_root(&mut self) -> Result<()> {
        if self.roots.is_empty() || self.root_selected >= self.roots.len() {
            return Ok(());
        }

        self.roots.remove(self.root_selected);
        self.root_selected = self.root_selected.min(self.roots.len().saturating_sub(1));
        storage::save_roots(&self.roots)?;

        if self.roots.is_empty() {
            self.stop();
            self.tracks.clear();
            self.path_order.clear();
            self.recent_indices_cache.clear();
            self.albums.clear();
            self.artists.clear();
            self.rebuild_playlist_indexes();
            self.search_results.clear();
            self.queue.clear();
            self.queue_base.clear();
            self.current = None;
            self.rescan_pending = false;
            self.status = "All library folders removed".to_owned();
        } else {
            self.begin_scan();
        }
        Ok(())
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current
            .and_then(|index| self.tracks.get(index))
            .or(self.browser_current.as_ref())
    }

    pub fn recent_indices(&self) -> &[usize] {
        &self.recent_indices_cache
    }

    fn refresh_recent_indices(&mut self) {
        self.recent_indices_cache = self
            .recent_paths
            .iter()
            .filter_map(|path| find_track_index(&self.tracks, &self.path_order, path))
            .collect();
    }

    pub fn position_seconds(&self) -> f64 {
        if self.state == PlaybackState::Stopped {
            0.0
        } else {
            self.audio.position().as_secs_f64()
        }
    }

    pub fn total_seconds(&self) -> Option<f64> {
        self.audio.total().map(|duration| duration.as_secs_f64())
    }
}

fn find_track_index(tracks: &[Track], path_order: &[usize], path: &Path) -> Option<usize> {
    path_order
        .binary_search_by(|index| tracks[*index].path.as_path().cmp(path))
        .ok()
        .map(|position| path_order[position])
}

fn compare_album_tracks(left: &Track, right: &Track) -> std::cmp::Ordering {
    left.disc_no
        .unwrap_or(0)
        .cmp(&right.disc_no.unwrap_or(0))
        .then_with(|| {
            left.track_no
                .unwrap_or(u32::MAX)
                .cmp(&right.track_no.unwrap_or(u32::MAX))
        })
        .then_with(|| compare_text(left.title.as_ref(), right.title.as_ref()))
        .then_with(|| left.path.cmp(&right.path))
}

fn compare_artist_tracks(left: &Track, right: &Track) -> std::cmp::Ordering {
    compare_text(left.album.as_ref(), right.album.as_ref())
        .then_with(|| compare_album_tracks(left, right))
}

fn compare_text(left: &str, right: &str) -> std::cmp::Ordering {
    if left.is_ascii() && right.is_ascii() {
        for (left, right) in left.bytes().zip(right.bytes()) {
            let ordering = left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase());
            if !ordering.is_eq() {
                return ordering;
            }
        }
        left.len().cmp(&right.len())
    } else {
        left.to_lowercase().cmp(&right.to_lowercase())
    }
}

fn matches_fields(fields: &[&str], tokens: &[String]) -> bool {
    tokens.iter().all(|token| {
        fields.iter().any(|field| {
            if field.is_ascii() && token.is_ascii() {
                contains_ascii_case_insensitive(field.as_bytes(), token.as_bytes())
            } else {
                field.to_lowercase().contains(token)
            }
        })
    })
}

fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    current
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

fn shuffle_slice<T>(values: &mut [T]) {
    if values.len() < 2 {
        return;
    }

    let mut state = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        ^ values.len() as u64;

    for index in (1..values.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let other = (state as usize) % (index + 1);
        values.swap(index, other);
    }
}
