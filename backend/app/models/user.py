from typing import Optional
from werkzeug.security import generate_password_hash, check_password_hash

class User:
    def __init__(
        self, 
        first_name: str, 
        last_name: str, 
        address: Optional[str] = None, 
        passport: Optional[str] = None,
        email: Optional[str] = None, 
        password_hash: Optional[str] = None
    ):
        self.first_name = first_name
        self.last_name = last_name
        self.address = address
        self.passport = passport
        self.is_verified = bool(address and passport)
        self.email = email
        self.password_hash = password_hash
        self.id = None

    def check_password(self, password: str) -> bool:
        if self.password_hash:
            return check_password_hash(self.password_hash, password)
        return False

    def __str__(self):
        return f"User({self.first_name} {self.last_name}, email={self.email}, verified={self.is_verified})"

class UserBuilder:
    def __init__(self):
        self.first_name: Optional[str] = None
        self.last_name: Optional[str] = None
        self.address: Optional[str] = None
        self.passport: Optional[str] = None
        self.email: Optional[str] = None
        self.password: Optional[str] = None

    def set_first_name(self, first_name: str):
        self.first_name = first_name
        return self

    def set_last_name(self, last_name: str):
        self.last_name = last_name
        return self

    def set_address(self, address: str):
        self.address = address
        return self

    def set_passport(self, passport: str):
        self.passport = passport
        return self

    def set_email(self, email: str):
        self.email = email
        return self

    def set_password(self, password: str):
        self.password = password
        return self

    def build(self):
        if not self.first_name or not self.last_name:
            raise ValueError("Имя и фамилия обязательны для создания пользователя")
        if self.email is None:
            raise ValueError("Email обязателен для регистрации")
        if self.password is None:
            raise ValueError("Пароль обязателен для регистрации")
        password_hash = generate_password_hash(self.password)
        return User(self.first_name, self.last_name, self.address, self.passport, self.email, password_hash)
