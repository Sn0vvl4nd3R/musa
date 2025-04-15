# Musa

Musa — это внутренняя логика музыкального стриминг-сервиса (аналог Spotify), реализованная в рамках курсового проекта. В данном проекте применяются порождающие паттерны и принципы SOLID для построения гибкой и расширяемой архитектуры.

---

## Структура проекта

---

## Установка и запуск

1. **Создание виртуального окружения**  
   Рекомендуется использовать Python 3.8 или выше:
   ```bash
   python -m venv venv
   ```
   Активируйте виртуальное окружение:
   - На Linux/Mac:
     ```bash
     source venv/bin/activate
     ```
   - На Windows:
     ```bash
     venv\Scripts\activate
     ```

2. **Установка зависимостей**  
   Установите необходимые пакеты:
   ```bash
   pip install -r requirements.txt
   ```

3. **Запуск проекта**  
   Вы можете запустить проект одним из двух способов:
   
   - **Через Makefile:**
     ```bash
     make run
     ```
     
   - **Непосредственно через Python:**
     ```bash
     python backend/app/main.py
     ```

---

## Описание проекта

Проект musa реализует основную логику музыкального стриминг-сервиса. Ключевые компоненты включают:
- **Модель пользователя:**  
  Пошаговое создание объекта с использованием паттерна Builder. Пользователь имеет обязательные поля (имя, фамилия) и опциональные (адрес, паспорт). На основе этих данных определяется, является ли пользователь верифицированным.
  
- **Модель музыкального трека:**  
  Представляет базовую информацию о треке — название, исполнитель и длительность.
  
- **Модель плейлиста:**  
  Позволяет создавать плейлисты и добавлять в них треки. Реализована с помощью паттерна Builder для гибкого создания объектов.

Архитектура проекта описана в файле `docs/architecture.md`, а связи между классами подробно отражены в UML-диаграмме `docs/uml/class_diagram.png`.

---

## Требования

- **Python:** Рекомендуется версия 3.8+
- **Зависимости:** Все необходимые библиотеки указаны в файле `requirements.txt`.

---

## Дополнительная информация

- **Makefile:**  
  Предоставляет удобные команды для установки зависимостей (`make install`), запуска проекта (`make run`) и очистки временных файлов (`make clean`).

- **Документация:**  
  Подробное описание архитектуры и используемых паттернов находится в файле `docs/architecture.md`.
