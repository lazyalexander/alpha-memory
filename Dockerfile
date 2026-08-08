# Python runtime with stdlib sqlite3 for alpha-memory storage.
FROM python:3.13-slim

WORKDIR /app

# sqlite3 CLI for inspection inside the container
RUN apt-get update \
    && apt-get install -y --no-install-recommends sqlite3 \
    && rm -rf /var/lib/apt/lists/*

COPY memory/ /app/memory/
COPY storage/ /app/storage/
COPY func.py test_func.py test_memory.py test_storage.py /app/

# Persistent DB directory (mount a volume here)
RUN mkdir -p /data
ENV ALPHA_MEMORY_DB=/data/alpha_memory.db
ENV PYTHONPATH=/app

CMD ["python", "-m", "unittest", "discover", "-v"]
