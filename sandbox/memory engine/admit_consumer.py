"""
Single-sandbox admit consumer (coroutine).

Responsibility
--------------
Drain durable pending remember_events in event_id order, materialize each
into admitted (admitted_index := event_id), mark done/failed.

One consumer per sandbox. Producers only append events; they wait until
admitted row exists before treating remember as complete.
"""

from __future__ import annotations

import asyncio
import importlib.util
import sqlite3
import threading
from pathlib import Path

from memory import AdmittedMemory
from storage.remember_log import list_pending, mark_done, mark_failed
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
    *,
    admit_lock: threading.Lock | None = None,
    max_items: int | None = None,
) -> list[AdmittedMemory]:
    """
    Synchronously process pending events in order (for tests / one-shot drain).
    """
    built: list[AdmittedMemory] = []
    pending = list_pending(conn)
    if max_items is not None:
        pending = pending[:max_items]
    for event in pending:
        try:
            row = _materialize(conn, event, admit_lock=admit_lock)
            mark_done(conn, event.event_id)
            built.append(row)
        except Exception as e:
            mark_failed(conn, event.event_id, str(e))
    return built


async def admit_consumer_loop(
    conn: sqlite3.Connection,
    *,
    admit_lock: threading.Lock | None = None,
    poll_interval: float = 0.01,
    stop_event: asyncio.Event | None = None,
) -> None:
    """
    Coroutine: repeatedly materialize pending remember events for one sandbox.
    """
    stop = stop_event if stop_event is not None else asyncio.Event()
    while not stop.is_set():
        # SQLite work off the event loop thread if needed; sync is fine for MVP.
        process_pending_once(conn, admit_lock=admit_lock)
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
    """
    Poll until admitted row exists (materialize complete) or timeout.

    Non-blocking style when used with short timeout + external consumer;
    for tests, call process_pending_once between polls or in another thread.
    """
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
    """Async poll until admitted_index = event_id exists."""
    loop = asyncio.get_event_loop()
    deadline = None if timeout is None else loop.time() + timeout
    while True:
        row = get_admitted(conn, event_id)
        if row is not None:
            return row
        if deadline is not None and loop.time() >= deadline:
            return None
        await asyncio.sleep(poll_interval)
