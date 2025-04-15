import time
from .logger import logger

def main():
    logger.info("Запуск приложения musa...")

    try:
        from .models.user import UserBuilder
        user_builder = UserBuilder()
        user_obj = (
            user_builder
            .set_first_name("Иван")
            .set_last_name("Иванов")
            .set_address("ул. Ленина, д.1")
            .set_passport("123456789")
            .build()
        )
        logger.info(f"Пользователь создан: {user_obj}")
    except Exception as e:
        logger.error("Ошибка при создании пользователя", exc_info=True)

    try:
        from .models.playlist import PlaylistBuilder
        playlist_builder = PlaylistBuilder()
        playlist = playlist_builder.set_name("Мои любимые треки").build()

        from .models.track import Track
        track1 = Track(title="Песня 1", artist="Исполнитель 1", duration=210)
        track2 = Track(title="Песня 2", artist="Исполнитель 2", duration=180)

        playlist.add_track(track1)
        playlist.add_track(track2)
        logger.info(f"Плейлист создан: {playlist}")
    except Exception as e:
        logger.error("Ошибка при создании плейлиста", exc_info=True)

    time.sleep(1)
    logger.info("Демонстрация завершена.")

if __name__ == '__main__':
    main()
