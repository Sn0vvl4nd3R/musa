struct AppState {
    name: String,
    state: PlaybackState,
    volume: i32,
    current_idx: Option<usize>,
    playlist: Vec<Track>,
}

impl AppState {
    fn new(name: String) -> Self {
        Self {
            name,
            state: PlaybackState::Stopped,
            volume: 50,
            current_idx: None,
            playlist: Vec::new(),
        }
    }

    fn print_state(&self) {
        println!("{}", self.name);
        println!("Volume: {}%", self.volume());
        println!("State: {}", self.state.as_str());
        match self.current_track() {
            Some(current_track) => {
                println!("Track: {} - {}", current_track.artist, current_track.title);
            }
            None => {
                println!("Track: no track");
            }
        }
    }

    fn set_volume(&mut self, volume: i32) {
        self.volume = volume.clamp(0, 100);
    }

    fn volume(&self) -> i32 {
        self.volume
    }

    fn play(&mut self) {
        if self.current_track().is_some() {
            self.state = PlaybackState::Playing;
        }
    }

    fn pause(&mut self) {
        self.state = match self.state {
            PlaybackState::Stopped => PlaybackState::Stopped,
            PlaybackState::Paused => PlaybackState::Paused,
            PlaybackState::Playing => PlaybackState::Paused,
        }
    }

    fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
    }

    fn toggle_play_pause(&mut self) {
        self.state = match self.state {
            PlaybackState::Stopped => {
                if self.current_track().is_some() {
                    PlaybackState::Playing
                } else {
                    PlaybackState::Stopped
                }
            },
            PlaybackState::Playing => PlaybackState::Paused,
            PlaybackState::Paused => PlaybackState::Playing,
        }
    }

    fn select_track(&mut self, idx: usize) {
        if self.playlist.get(idx).is_some() {
            self.current_idx = Some(idx);
            self.play();
        }
    }

    fn clear_current_track(&mut self) {
        self.current_idx = None;
        self.stop();
    }

    fn current_track(&self) -> Option<&Track> {
        match self.current_idx {
            Some(idx) => self.playlist.get(idx),
            None => None,
        }
    }

    fn add_track(&mut self, track: Track) {
        self.playlist.push(track);
    }

    fn playlist_len(&self) -> usize {
        self.playlist.len()
    }

    fn print_playlist(&self) {
        println!("Playlist:");
        for (idx, track) in self.playlist.iter().enumerate() {
            println!("{}. {} - {}", (idx + 1), track.artist, track.title);
        }
    }

    fn next_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }

        match self.current_idx {
            None => self.select_track(0),
            Some(idx) => {
                if idx == self.playlist.len() - 1 {
                    self.select_track(0);
                } else {
                    self.select_track(idx + 1);
                }
            }
        }
    }

    fn previous_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }

        match self.current_idx {
            None => self.select_track(0),
            Some(idx) => {
                if idx == 0 {
                    self.select_track(self.playlist.len() - 1);
                } else {
                    self.select_track(idx - 1);
                }
            }
        }
    }
}

enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

impl PlaybackState {
    fn as_str(&self) -> &'static str {
        match self {
            PlaybackState::Stopped => "stopped",
            PlaybackState::Playing => "playing",
            PlaybackState::Paused => "paused",
        }
    }
}

struct Track {
    title: String,
    artist: String,
}

fn main() {
    let mut app = AppState::new(String::from("Musa"));
    let track0 = Track { title: String::from("Get Lucky"), artist: String::from("Daft Punk") };
    let track1 = Track { title: String::from("Genesis"), artist: String::from("Justice") };
    let track2 = Track { title: String::from("Nightcall"), artist: String::from("Kavinsky") };

    app.toggle_play_pause();
    app.print_state();

    app.add_track(track0);
    app.add_track(track1);
    app.add_track(track2);

    app.select_track(0);
    app.print_state();

    app.next_track();
    app.print_state();

    app.next_track();
    app.print_state();

    app.next_track();
    app.print_state();

    app.next_track();
    app.print_state();

    app.clear_current_track();
    app.print_state();
}
