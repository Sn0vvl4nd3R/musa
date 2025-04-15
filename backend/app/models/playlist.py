from typing import Optional

class Playlist:
    def __init__(self, name: str, owner_id: Optional[int] = None):
        self.name = name
        self.tracks = []
        self.owner_id = owner_id
        self.id = None

    def add_track(self, track):
        self.tracks.append(track)

    def __str__(self):
        track_titles = ", ".join(track.title for track in self.tracks)
        return f"Playlist('{self.name}', Tracks: [{track_titles}], owner_id={self.owner_id})"

class PlaylistBuilder:
    def __init__(self):
        self.name = "Без названия"
        self.tracks = []
        self.owner_id = None

    def set_name(self, name: str):
        self.name = name
        return self

    def set_owner(self, owner_id: int):
        self.owner_id = owner_id
        return self

    def add_track(self, track):
        self.tracks.append(track)
        return self

    def build(self):
        playlist = Playlist(self.name, self.owner_id)
        for track in self.tracks:
            playlist.add_track(track)
        return playlist
