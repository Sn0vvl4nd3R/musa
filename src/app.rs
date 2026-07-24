use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    Result,
    audio::AudioEngine,
    library::{self, DirectoryEntry, ScanEvent, Track},
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

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
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
    Folders,
}

impl View {
    pub const ALL: [Self; 6] = [
        Self::Home,
        Self::Search,
        Self::Songs,
        Self::Albums,
        Self::Artists,
        Self::Folders,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Search => "Search",
            Self::Songs => "Songs",
            Self::Albums => "Albums",
            Self::Artists => "Artists",
            Self::Folders => "Folders",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailView {
    Album(usize),
    Artist(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderFocus {
    Roots,
    Browser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchItem {
    Artist(usize),
    Album(usize),
    Track(usize),
}

#[derive(Clone, Debug)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<usize>,
    pub duration: Duration,
}

#[derive(Clone, Debug)]
pub struct Artist {
    pub name: String,
    pub tracks: Vec<usize>,
    pub album_count: usize,
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
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,

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

    pub browser_dir: PathBuf,
    pub browser_entries: Vec<DirectoryEntry>,
    pub folder_focus: FolderFocus,
    pub root_selected: usize,
    pub browser_selected: usize,

    pub scan_phase: ScanPhase,
    scan_rx: Option<Receiver<ScanEvent>>,
    rescan_pending: bool,
    audio: AudioEngine,
}

impl App {
    pub fn new() -> Result<Self> {
        let roots = storage::load_roots();
        let theme = storage::load_theme();
        let browser_dir = storage::home_dir();
        let browser_entries = library::read_subdirectories(&browser_dir).unwrap_or_default();
        let volume = 70;
        let view = if roots.is_empty() {
            View::Folders
        } else {
            View::Home
        };

        let mut app = Self {
            roots,
            tracks: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),
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
            browser_dir,
            browser_entries,
            folder_focus: FolderFocus::Browser,
            root_selected: 0,
            browser_selected: 0,
            scan_phase: ScanPhase::Idle,
            scan_rx: None,
            rescan_pending: false,
            audio: AudioEngine::new(volume)?,
        };

        if !app.roots.is_empty() {
            app.begin_scan();
        }

        Ok(app)
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
            KeyCode::Char('6') | KeyCode::Char('o') => {
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
            KeyCode::Char('a') if self.view == View::Folders => self.add_browser_root(),
            KeyCode::Char('d') if self.view == View::Folders => self.remove_selected_root(),
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

    pub fn tick(&mut self) {
        self.poll_scan();

        if self.state == PlaybackState::Playing && self.audio.is_empty() {
            if let Err(error) = self.next_track(true) {
                self.status = error.to_string();
                self.state = PlaybackState::Stopped;
            }
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

    fn poll_scan(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;

        if let Some(receiver) = self.scan_rx.as_ref() {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        let mut finished = false;
        for event in events {
            match event {
                ScanEvent::Progress { done, total } => {
                    self.scan_phase = ScanPhase::Reading { done, total };
                    self.status = format!("Reading metadata: {done}/{total}");
                }
                ScanEvent::Finished(result) => {
                    finished = true;
                    self.scan_phase = ScanPhase::Idle;
                    match result {
                        Ok(tracks) if !self.roots.is_empty() => self.apply_scan_result(tracks),
                        Ok(_) => {
                            self.status = "All library folders removed".to_owned();
                        }
                        Err(error) => self.status = format!("Library scan failed: {error}"),
                    }
                }
            }
        }

        if finished || disconnected {
            self.scan_rx = None;
            if disconnected && !finished {
                self.scan_phase = ScanPhase::Idle;
                self.status = "Library scan stopped unexpectedly".to_owned();
            }
            if self.rescan_pending {
                self.rescan_pending = false;
                self.begin_scan();
            }
        }
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

        let path_to_index: HashMap<PathBuf, usize> = self
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| (track.path.clone(), index))
            .collect();

        self.current = old_current_path
            .as_ref()
            .and_then(|path| path_to_index.get(path).copied());
        self.queue_base = old_queue_paths
            .iter()
            .filter_map(|path| path_to_index.get(path).copied())
            .collect();

        if self.queue_base.is_empty() {
            if let Some(current) = self.current {
                self.queue_base.push(current);
            }
        }
        self.rebuild_queue_order();

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
        let mut album_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut artist_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();

        for (index, track) in self.tracks.iter().enumerate() {
            let album_key = format!(
                "{}\0{}",
                track.album_dir.to_string_lossy().to_lowercase(),
                track.album.to_lowercase()
            );
            album_map.entry(album_key).or_default().push(index);
            artist_map
                .entry(track.artist.to_lowercase())
                .or_default()
                .push(index);
        }

        let mut albums: Vec<Album> = album_map
            .into_values()
            .map(|mut tracks| {
                tracks.sort_by(|left, right| compare_album_tracks(&self.tracks[*left], &self.tracks[*right]));
                let first = &self.tracks[tracks[0]];
                let artists: BTreeSet<&str> = tracks
                    .iter()
                    .filter_map(|index| {
                        let artist = self.tracks[*index].album_artist.trim();
                        (!artist.is_empty()).then_some(artist)
                    })
                    .collect();
                let artist = if artists.len() == 1 {
                    artists.iter().next().copied().unwrap_or(&first.artist).to_owned()
                } else {
                    "Various Artists".to_owned()
                };
                let duration = tracks.iter().fold(Duration::ZERO, |total, index| {
                    total.saturating_add(self.tracks[*index].duration.unwrap_or_default())
                });
                Album {
                    title: first.album.clone(),
                    artist,
                    tracks,
                    duration,
                }
            })
            .collect();

        albums.sort_by(|left, right| {
            left.artist
                .to_lowercase()
                .cmp(&right.artist.to_lowercase())
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        });

        let mut artists: Vec<Artist> = artist_map
            .into_values()
            .map(|mut tracks| {
                tracks.sort_by(|left, right| compare_artist_tracks(&self.tracks[*left], &self.tracks[*right]));
                let name = self.tracks[tracks[0]].artist.clone();
                let album_count = tracks
                    .iter()
                    .map(|index| {
                        format!(
                            "{}\0{}",
                            self.tracks[*index].album_dir.to_string_lossy().to_lowercase(),
                            self.tracks[*index].album.to_lowercase()
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

        for (index, artist) in self.artists.iter().enumerate() {
            if matches_tokens(&artist.name.to_lowercase(), &tokens) {
                self.search_results.push(SearchItem::Artist(index));
            }
        }
        for (index, album) in self.albums.iter().enumerate() {
            let text = format!("{}\n{}", album.title, album.artist).to_lowercase();
            if matches_tokens(&text, &tokens) {
                self.search_results.push(SearchItem::Album(index));
            }
        }
        for (index, track) in self.tracks.iter().enumerate() {
            if matches_tokens(&track.search_text, &tokens) {
                self.search_results.push(SearchItem::Track(index));
            }
        }
    }

    fn activate_selected(&mut self) -> Result<()> {
        match self.view {
            View::Home => {
                let recent = self.recent_indices();
                if let Some(track) = recent.get(self.selected).copied() {
                    self.play_queue(recent, track)?;
                }
            }
            View::Search => {
                let Some(item) = self.search_results.get(self.selected).copied() else {
                    return Ok(());
                };
                match item {
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
            View::Folders => self.activate_folder()?,
        }
        Ok(())
    }

    fn play_queue(&mut self, queue: Vec<usize>, track: usize) -> Result<()> {
        if queue.is_empty() || self.tracks.get(track).is_none() {
            return Ok(());
        }
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
        self.current = Some(index);
        self.queue_pos = self.queue.iter().position(|queued| *queued == index);
        self.state = PlaybackState::Playing;
        self.status = format!("Playing {title}");

        self.recent_paths.retain(|recent| recent != &path);
        self.recent_paths.insert(0, path);
        self.recent_paths.truncate(50);
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
                if let Some(current) = self.current {
                    if self.queue_base.is_empty() {
                        self.queue_base.push(current);
                        self.rebuild_queue_order();
                    }
                    self.play_track(current)?;
                } else if let Some((queue, track)) = self.queue_for_selection() {
                    self.play_queue(queue, track)?;
                } else {
                    self.status = "Select a song, album, or artist first".to_owned();
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
                Some((queue, track))
            }
            View::Search => {
                let item = self.search_results.get(self.selected)?;
                match *item {
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
            View::Folders => None,
        }
    }

    fn next_track(&mut self, automatic: bool) -> Result<()> {
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
        self.queue = self.queue_base.clone();
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

    fn toggle_theme(&mut self) {
        self.theme = match self.theme {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
        if let Err(error) = storage::save_theme(self.theme) {
            self.status = format!("Theme changed, but could not be saved: {error}");
        } else {
            self.status = format!("{} theme", self.theme.label());
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
                    self.browser_dir = entry.path;
                    self.refresh_browser();
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
        match library::read_subdirectories(&self.browser_dir) {
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
            self.albums.clear();
            self.artists.clear();
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
        self.current.and_then(|index| self.tracks.get(index))
    }

    pub fn recent_indices(&self) -> Vec<usize> {
        let path_to_index: HashMap<PathBuf, usize> = self
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| (track.path.clone(), index))
            .collect();
        self.recent_paths
            .iter()
            .filter_map(|path| path_to_index.get(path).copied())
            .collect()
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

fn compare_album_tracks(left: &Track, right: &Track) -> std::cmp::Ordering {
    left.disc_no
        .unwrap_or(0)
        .cmp(&right.disc_no.unwrap_or(0))
        .then_with(|| {
            left.track_no
                .unwrap_or(u32::MAX)
                .cmp(&right.track_no.unwrap_or(u32::MAX))
        })
        .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        .then_with(|| left.path.cmp(&right.path))
}

fn compare_artist_tracks(left: &Track, right: &Track) -> std::cmp::Ordering {
    left.album
        .to_lowercase()
        .cmp(&right.album.to_lowercase())
        .then_with(|| compare_album_tracks(left, right))
}

fn matches_tokens(haystack: &str, tokens: &[String]) -> bool {
    tokens.iter().all(|token| haystack.contains(token))
}

fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    current
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

fn shuffle_slice(values: &mut [usize]) {
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
