"""
remember — durable segment log + materialize to Admitted (truth in SQLite).

Flow
----
1. admit_validate (stub True)
2. EventIdAllocator.next_id()  (file counter, not SQL autoincrement)
3. FileRememberLog.append  (segment files, 1000 events/file, fsync)
4. Consumer replays log from checkpoint → INSERT admitted(event_id=…)
5. Complete only when admitted row exists (DB commit ok)

Producers only append log (+ optional wait). No SQL in the submit path
beyond optional shared conn for materialize wait.
"""

from __future__ import annotations

import importlib.util
import sqlite3
import threading
from dataclasses import dataclass
from pathlib import Path

from memory import AdmittedMemory
from storage.remember_log import FileRememberLog, RememberEvent
from storage.sqlite_store import get_admitted
from util.event_id import EventIdAllocator


def _load_consumer():
    path = (
        Path(__file__).resolve().parents[2]
        / "sandbox"
        / "memory engine"
        / "admit_consumer.py"
    )
    spec = importlib.util.spec_from_file_location(
        "alpha_memory_admit_consumer", path
    )
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load admit_consumer from {path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_consumer = _load_consumer()


@dataclass(frozen=True, slots=True)
class RememberTicket:
    """event_id becomes admitted_index only after successful materialize."""

    event_id: int


def admit_validate(data: str, from_agent: int) -> bool:
    """MVP stub: always True."""
    return True


def submit_remember_event(
    log: FileRememberLog,
    allocator: EventIdAllocator,
    *,
    data: str,
    from_agent: int,
    id_lock: threading.Lock | None = None,
) -> RememberTicket | None:
    """
    Assign event_id, append to segment log (durable). No Admitted row yet.
    """
    if not admit_validate(data, from_agent):
        return None

    def _go() -> int:
        event_id = allocator.next_id()
        log.append(
            RememberEvent(
                event_id=event_id, data=data, from_agent=from_agent
            )
        )
        return event_id

    if id_lock is not None:
        with id_lock:
            event_id = _go()
    else:
        event_id = _go()
    return RememberTicket(event_id=event_id)


def remember(
    conn: sqlite3.Connection,
    log: FileRememberLog,
    allocator: EventIdAllocator,
    *,
    data: str,
    from_agent: int,
    admit_lock: threading.Lock | None = None,
    id_lock: threading.Lock | None = None,
) -> AdmittedMemory | None:
    """
    Submit log event, process pending from checkpoint, require admitted row.
    """
    ticket = submit_remember_event(
        log,
        allocator,
        data=data,
        from_agent=from_agent,
        id_lock=id_lock,
    )
    if ticket is None:
        return None
    _consumer.process_pending_once(conn, log, admit_lock=admit_lock)
    row = get_admitted(conn, ticket.event_id)
    if row is None:
        raise RuntimeError(
            f"remember incomplete: no admitted row for event_id={ticket.event_id}"
        )
    return row


async def remember_async(
    conn: sqlite3.Connection,
    log: FileRememberLog,
    allocator: EventIdAllocator,
    *,
    data: str,
    from_agent: int,
    admit_lock: threading.Lock | None = None,
    id_lock: threading.Lock | None = None,
    timeout: float | None = 5.0,
) -> AdmittedMemory | None:
    ticket = submit_remember_event(
        log,
        allocator,
        data=data,
        from_agent=from_agent,
        id_lock=id_lock,
    )
    if ticket is None:
        return None
    _consumer.process_pending_once(conn, log, admit_lock=admit_lock)
    return await _consumer.wait_until_admitted_async(
        conn, ticket.event_id, timeout=timeout
    )
