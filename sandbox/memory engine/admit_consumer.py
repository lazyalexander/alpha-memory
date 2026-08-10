"""
Single-sandbox admit consumer (coroutine).

Replays FileRememberLog from checkpoint, materializes admitted
(admitted_index := event_id), advances checkpoint. Truth = SQLite admitted.
"""

from __future__ import annotations

import asyncio
import importlib.util
import sqlite3
import threading
from pathlib import Path

from memory import AdmittedMemory
from storage.remember_log import FileRememberLog
from storage.sqlite_store import get_admitted


def _load_materialize():
    path = Path(__file__).resolve().parent / "materialize.py"
    spec = importlib.util.spec_from_file_location(
        "alpha_memory_materialize", path
    )
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load materialize from {path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.materialize_remember_event


_materialize = _load_materialize()


def process_pending_once(
    conn: sqlite3.Connection,
    log: FileRememberLog,
    *,
    admit_lock: threading.Lock | None = None,
    max_items: int | None = None,
) -> list[AdmittedMemory]:
    """
    Process events after checkpoint in order.
    On success: admitted exists + checkpoint advanced.
    Idempotent if admitted already present.
    """
    built: list[AdmittedMemory] = []
    cp = log.load_checkpoint()
    n = 0
    for event in log.iter_from(cp):
        if max_items is not None and n >= max_items:
            break
        row = _materialize(conn, event, admit_lock=admit_lock)
        # Only advance checkpoint after successful materialize (truth in DB).
        if get_admitted(conn, event.event_id) is None:
            raise RuntimeError(
                f"materialize left no admitted row for event_id={event.event_id}"
            )
        log.save_checkpoint(event.event_id)
        built.append(row)
        n += 1
    return built


async def admit_consumer_loop(
    conn: sqlite3.Connection,
    log: FileRememberLog,
    *,
    admit_lock: threading.Lock | None = None,
    poll_interval: float = 0.01,
    stop_event: asyncio.Event | None = None,
) -> None:
    """Coroutine: one consumer per sandbox, replay log → SQLite."""
    stop = stop_event if stop_event is not None else asyncio.Event()
    while not stop.is_set():
        process_pending_once(conn, log, admit_lock=admit_lock)
        try:
            await asyncio.wait_for(stop.wait(), timeout=poll_interval)
        except asyncio.TimeoutError:
            continue


def wait_until_admitted(
    conn: sqlite3.Connection,
    event_id: int,
    *,
    timeout: float | None = 5.0,
    poll_interval: float = 0.01,
) -> AdmittedMemory | None:
    import time

    deadline = None if timeout is None else time.monotonic() + timeout
    while True:
        row = get_admitted(conn, event_id)
        if row is not None:
            return row
        if deadline is not None and time.monotonic() >= deadline:
            return None
        time.sleep(poll_interval)


async def wait_until_admitted_async(
    conn: sqlite3.Connection,
    event_id: int,
    *,
    timeout: float | None = 5.0,
    poll_interval: float = 0.01,
) -> AdmittedMemory | None:
    loop = asyncio.get_event_loop()
    deadline = None if timeout is None else loop.time() + timeout
    while True:
        row = get_admitted(conn, event_id)
        if row is not None:
            return row
        if deadline is not None and loop.time() >= deadline:
            return None
        await asyncio.sleep(poll_interval)
