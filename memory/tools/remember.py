"""
remember — single ingress memory tool (AdmittedMemory only).

Responsibility
--------------
Queue-shaped admission of legal payload into a sandbox DB. This is the only
tool that introduces new ``data``. It does **not** talk SQL directly: it
validates (stub), then calls Memory Engine ``build_admitted``.

Decoupling
----------
- memory/admitted.py     — data shape only
- memory/tools/remember  — tool policy + admit queue (this file)
- memory engine/engine   — build + lock + BEGIN IMMEDIATE
- storage/sqlite_store   — INSERT

Queue
-----
Multi-producer: ``submit_remember`` only.
Single consumer: ``process_remember_queue`` drains serially (write limit =
what is on the queue).
Sync helper: ``remember`` uses a one-shot private queue (same code path).

Validation
----------
``admit_validate`` currently always returns True (placeholder).
"""

from __future__ import annotations

import importlib.util
import sqlite3
import threading
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from queue import Empty, Queue

from memory import AdmittedMemory

IndexAcquire = Callable[[], int | None]


def _load_build_admitted():
    path = (
        Path(__file__).resolve().parents[2]
        / "sandbox"
        / "memory engine"
        / "engine.py"
    )
    spec = importlib.util.spec_from_file_location(
        "alpha_memory_engine_for_remember", path
    )
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load engine from {path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.build_admitted


_build_admitted = _load_build_admitted()


@dataclass(frozen=True, slots=True)
class RememberRequest:
    """One queued admit intent (not yet a domain mem)."""

    data: str
    from_agent: int


def admit_validate(data: str, from_agent: int) -> bool:
    """
    Whether payload may enter as AdmittedMemory.

    MVP stub: always True. Real checks added later.
    """
    return True


def submit_remember(queue: Queue[RememberRequest], request: RememberRequest) -> None:
    """Enqueue an admit intent. Does not persist."""
    queue.put(request)


def process_remember_queue(
    queue: Queue[RememberRequest],
    conn: sqlite3.Connection,
    *,
    acquire_index: IndexAcquire,
    admit_lock: threading.Lock,
    max_items: int | None = None,
) -> list[AdmittedMemory]:
    """
    Drain the queue serially (up to max_items, or until empty).

    Only dequeued requests are written — the queue bounds write load.
    Order of successful builds follows queue order under admit_lock.
    """
    built: list[AdmittedMemory] = []
    n = 0
    while max_items is None or n < max_items:
        try:
            req = queue.get_nowait()
        except Empty:
            break
        try:
            if not admit_validate(req.data, req.from_agent):
                continue
            row = _build_admitted(
                conn,
                data=req.data,
                from_agent=req.from_agent,
                acquire_index=acquire_index,
                admit_lock=admit_lock,
            )
            built.append(row)
            n += 1
        finally:
            queue.task_done()
    return built


def remember(
    conn: sqlite3.Connection,
    *,
    data: str,
    from_agent: int,
    acquire_index: IndexAcquire,
    admit_lock: threading.Lock,
) -> AdmittedMemory | None:
    """
    Sync ingress: validate, one-shot queue, process one → complete AdmittedMemory.

    Returns None if validation fails (stub never fails).
    """
    if not admit_validate(data, from_agent):
        return None
    q: Queue[RememberRequest] = Queue()
    submit_remember(q, RememberRequest(data=data, from_agent=from_agent))
    built = process_remember_queue(
        q,
        conn,
        acquire_index=acquire_index,
        admit_lock=admit_lock,
        max_items=1,
    )
    return built[0] if built else None
