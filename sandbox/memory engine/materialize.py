"""
Materialize remember log events into AdmittedMemory (domain truth).

admitted_index := event_id (explicit). Idempotent if row already exists.
"""

from __future__ import annotations

import importlib.util
import sqlite3
import threading
from pathlib import Path

from memory import AdmittedMemory
from storage.remember_log import RememberEvent
from storage.sqlite_store import get_admitted


def _load_build_admitted():
    path = Path(__file__).resolve().parent / "engine.py"
    spec = importlib.util.spec_from_file_location(
        "alpha_memory_engine_materialize", path
    )
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load engine from {path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.build_admitted


_build_admitted = _load_build_admitted()


def materialize_remember_event(
    conn: sqlite3.Connection,
    event: RememberEvent,
    *,
    admit_lock: threading.Lock | None = None,
) -> AdmittedMemory:
    """Ensure AdmittedMemory(event_id) exists in SQLite; return it."""
    existing = get_admitted(conn, event.event_id)
    if existing is not None:
        return existing
    return _build_admitted(
        conn,
        data=event.data,
        from_agent=event.from_agent,
        acquire_index=lambda: event.event_id,
        admit_lock=admit_lock,
    )
