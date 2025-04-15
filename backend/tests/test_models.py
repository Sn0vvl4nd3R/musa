import sys
import os

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

import unittest
from app.models.user import UserBuilder
from app.models.track import Track
from app.models.playlist import PlaylistBuilder

class TestUserModel(unittest.TestCase):
    def test_create_valid_user(self):
        builder = UserBuilder()
        user = (builder
                .set_first_name("Test")
                .set_last_name("User")
                .set_address("Test Address")
                .set_passport("TestPassport")
                .build())
        self.assertEqual(user.first_name, "Test")
        self.assertEqual(user.last_name, "User")
        self.assertTrue(user.is_verified)

    def test_missing_first_name_raises_error(self):
        builder = UserBuilder()
        builder.set_last_name("User")
        with self.assertRaises(ValueError):
            builder.build()

    def test_missing_last_name_raises_error(self):
        builder = UserBuilder()
        builder.set_first_name("Test")
        with self.assertRaises(ValueError):
            builder.build()

class TestTrackModel(unittest.TestCase):
    def test_track_str(self):
        track = Track(title="Song", artist="Artist", duration=120)
        expected_str = "Track('Song' by Artist, 120s)"
        self.assertEqual(str(track), expected_str)

class TestPlaylistModel(unittest.TestCase):
    def test_playlist_builder_creates_empty_playlist(self):
        builder = PlaylistBuilder()
        playlist = builder.set_name("Favorites").build()
        self.assertEqual(playlist.name, "Favorites")
        self.assertEqual(len(playlist.tracks), 0)

    def test_adding_tracks_to_playlist(self):
        builder = PlaylistBuilder()
        playlist = builder.set_name("Favorites").build()
        track1 = Track(title="Song1", artist="Artist1", duration=200)
        track2 = Track(title="Song2", artist="Artist2", duration=180)
        playlist.add_track(track1)
        playlist.add_track(track2)
        self.assertEqual(len(playlist.tracks), 2)
        self.assertIn(track1, playlist.tracks)
        self.assertIn(track2, playlist.tracks)

if __name__ == "__main__":
    unittest.main()
