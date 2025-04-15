class MusaError(Exception):
    def __init__(self, message: str = "Произошла ошибка в проекте musa"):
        super().__init__(message)
        self.message = message

class InvalidUserError(MusaError):
    def __init__(self, message: str = "Неверные данные для создания пользователя"):
        super().__init__(message)

class PlaylistError(MusaError):
    def __init__(self, message: str = "Ошибка при работе с плейлистом"):
        super().__init__(message)

class TrackError(MusaError):
    def __init__(self, message: str = "Ошибка при работе с треком"):
        super().__init__(message)
