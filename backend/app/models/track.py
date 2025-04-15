class Track:
    def __init__(self, title: str, artist: str, duration: int):
        self.title = title
        self.artist = artist
        self.duration = duration

    def __str__(self):
        return f"Track('{self.title}' by {self.artist}, {self.duration}s)"
