class Config:
    SUSPICIOUS_USER_STREAM_LIMIT = 2

    DEFAULT_VOLUME = 50
    MAX_VOLUME = 100
    MIN_VOLUME = 0

    STREAM_BUFFER_TIMEOUT = 5

    APP_NAME = "Musa"
    VERSION = "1.0.0"

    DATABASE_URI = "sqlite:///music_streaming.db"

    @staticmethod
    def print_config():
        print("Конфигурация приложения:")
        print(f"APP_NAME: {Config.APP_NAME}")
        print(f"VERSION: {Config.VERSION}")
        print(f"DEFAULT_VOLUME: {Config.DEFAULT_VOLUME}")
        print(f"MAX_VOLUME: {Config.MAX_VOLUME}")
        print(f"STREAM_BUFFER_TIMEOUT: {Config.STREAM_BUFFER_TIMEOUT}")
        print(f"Database URI: {Config.DATABASE_URI}")
