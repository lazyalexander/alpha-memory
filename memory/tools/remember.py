"""
remember — single ingress tool: durable event, then materialize to Admitted.

Flow
----
1. admit_validate (stub True)
2. allocate event_id via EventIdAllocator (not SQL autoincrement)
3. append remember_event (pending) — durable ordered intent
4. consumer materializes admitted with admitted_index := event_id
5. complete only when admitted row exists (commit succeeded)

Producers only write events (+ optional non-blocking wait for admitted).
SQL details stay in storage/; build stays in engine/; this file orchestrates.
"""

from __future__ import annotations

import importlib.util
import sqlite3
import threading
from dataclasses import dataclass
from pathlib import Path

from memory import AdmittedMemory
from storage.remember_log import append_pending, get_event
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
    """
    Handle after durable event append.

    event_id will become admitted_index after successful materialize.
    Complete only when get_admitted(conn, event_id) is not None.
    """

    event_id: int


def admit_validate(data: str, from_agent: int) -> bool:
    """MVP stub: always True."""
    return True


def submit_remember_event(
    conn: sqlite3.Connection,
    allocator: EventIdAllocator,
    *,
    data: str,
    from_agent: int,
    id_lock: threading.Lock | None = None,
) -> RememberTicket | None:
    """
    Validate, assign event_id (app-level), persist pending remember_event.

    Does not create AdmittedMemory. Returns None if validation fails.
    id_lock: optional lock around next_id + append for multi-threaded producers.
    """
    if not admit_validate(data, from_agent):
        return None

    def _assign_and_log() -> int:
        event_id = allocator.next_id()
        append_pending(
            conn,
            event_id=event_id,
            data=data,
            from_agent=from_agent,
        )
        return event_id

    if id_lock is not None:
        with id_lock:
            event_id = _assign_and_log()
    else:
        event_id = _assign_and_log()
    return RememberTicket(event_id=event_id)


def remember(
    conn: sqlite3.Connection,
    allocator: EventIdAllocator,
    *,
    data: str,
    from_agent: int,
    admit_lock: threading.Lock | None = None,
    id_lock: threading.Lock | None = None,
) -> AdmittedMemory | None:
    """
    Sync path: submit durable event, then process pending until this event
    is materialized (admitted row exists). Failure => None / raise.

    Completion criterion: admitted row with admitted_index == event_id.
    """
    ticket = submit_remember_event(
        conn,
        allocator,
        data=data,
        from_agent=from_agent,
        id_lock=id_lock,
    )
    if ticket is None:
        return None
    # Drain pending in order (includes this event); single-sandbox consumer.
    _consumer.process_pending_once(conn, admit_lock=admit_lock)
    row = get_admitted(conn, ticket.event_id)
    if row is None:
        ev = get_event(conn, ticket.event_id)
        if ev is not None and ev.status == "failed":
            raise RuntimeError(
                f"remember materialize failed for event_id={ticket.event_id}: {ev.error}"
            )
        raise RuntimeError(
            f"remember incomplete: no admitted row for event_id={ticket.event_id}"
        )
    return row


async def remember_async(
    conn: sqlite3.Connection,
    allocator: EventIdAllocator,
    *,
    data: str,
    from_agent: int,
    admit_lock: threading.Lock | None = None,
    id_lock: threading.Lock | None = None,
    timeout: float | None = 5.0,
) -> AdmittedMemory | None:
    """
    Submit event, kick one process pass, then async-wait until admitted exists.
    Prefer a running admit_consumer_loop in production; this still works alone.
    """
    ticket = submit_remember_event(
        conn,
        allocator,
        data=data,
        from_agent=from_agent,
        id_lock=id_lock,
    )
    if ticket is None:
        return None
    _consumer.process_pending_once(conn, admit_lock=admit_lock)
    return await _consumer.wait_until_admitted_async(
        conn, ticket.event_id, timeout=timeout
    )
