.PHONY: install run-backend run-frontend run clean

install:
	@if [ ! -d "venv" ]; then \
		echo "Создаю виртуальное окружение..."; \
		python -m venv venv; \
	else \
		echo "Виртуальное окружение уже существует"; \
	fi
	@echo "Устанавливаем зависимости..."
	@venv/bin/pip install -r requirements.txt

run-backend:
	@echo "Запуск backend..."
	@PYTHONPATH=. venv/bin/python -m backend.app.api &

run-frontend:
	@echo "Запуск статического сервера для фронтенда на порту 8000..."
	@cd frontend && python -m http.server 8000

run: run-backend run-frontend

clean:
	@echo "Очистка временных файлов..."
	@find . -type f -name '*.pyc' -delete
	@find . -type d -name '__pycache__' -delete

