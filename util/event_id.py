"""
Sandbox-local event id allocation (mechanism-agnostic).

Properties (guaranteed by this module's contract, not by SQLite AUTOINCREMENT):
- monotonic increasing within one allocator instance / durable store
- unique
- crash-recoverable when using a durable backend (file fsync)

Remember path: event_id is assigned here; admitted_index := event_id on materialize.
"""

from __future__ import annotations

import os
import threading
from pathlib import Path
from typing import Protocol


class EventIdAllocator(Protocol):
    """Allocate the next remember-event id for one sandbox."""

    def next_id(self) -> int:
        """Return a new id >= 1; durable before return when backend is durable."""
        ...


class FileEventIdAllocator:
    """
    Persist max event id in a side file (not SQL rowid / AUTOINCREMENT).

    File format: single integer text, atomic replace after write.
    """

    def __init__(self, path: str | Path, *, lock: threading.Lock | None = None) -> None:
        self._path = Path(path)
        self._lock = lock if lock is not None else threading.Lock()

    def next_id(self) -> int:
        with self._lock:
            self._path.parent.mkdir(parents=True, exist_ok=True)
            current = 0
            if self._path.exists():
                text = self._path.read_text(encoding="utf-8").strip()
                if text:
                    current = int(text)
            new_id = current + 1
            tmp = self._path.with_suffix(self._path.suffix + ".tmp")
            with open(tmp, "w", encoding="utf-8") as f:
                f.write(str(new_id))
                f.flush()
                os.fsync(f.fileno())
            os.replace(tmp, self._path)
            return new_id
