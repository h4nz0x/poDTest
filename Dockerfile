# syntax=docker/dockerfile:1

ARG PYTHON_VERSION=3.11
FROM python:${PYTHON_VERSION}-slim AS base

# Prevent Python from writing pyc files
ENV PYTHONDONTWRITEBYTECODE=1

# Keep Python from buffering stdout/stderr
ENV PYTHONUNBUFFERED=1

# Create non-root user
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --no-cache-dir -r requirements.txt

COPY main.py .

USER appuser
EXPOSE 3000
CMD ["gunicorn", "-w", "4", "-k", "uvicorn.workers.UvicornWorker", "--timeout", "120", "--keep-alive", "5", "main:app", "-b", "0.0.0.0:3000"]