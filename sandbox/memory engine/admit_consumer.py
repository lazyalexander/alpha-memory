"""
Single-sandbox admit consumer (coroutine) + startup recover.

Replays FileRememberLog from checkpoint, materializes admitted
(admitted_index := event_id), advances checkpoint. Truth = SQLite admitted.

Startup
-------
``inspect_sandbox`` compares log / checkpoint / allocator / DB.
``recover_sandbox`` runs inspect, then automatic replay if needed, then
re-inspects. Call once when opening a sandbox before serving traffic.
"""

from __future__ import annotations

import asyncio
import importlib.util
import sqlite3
import threading
from pathlib import Path
from typing import NamedTuple

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


# --- startup inspect / recover -------------------------------------------------


class RecoverReport(NamedTuple):
    """Result of inspect_sandbox / recover_sandbox."""

    checkpoint_before: int
    max_log_event_id: int
    allocator_event_id: int | None
    pending_event_count: int
    missing_admitted_count: int
    needs_replay: bool
    unclean_shutdown_suspected: bool
    replayed: int
    checkpoint_after: int
    warnings: tuple[str, ...]


def _read_allocator_id(event_id_path: Path | None) -> int | None:
    if event_id_path is None:
        return None
    path = Path(event_id_path)
    if not path.exists():
        return 0
    text = path.read_text(encoding="utf-8").strip()
    if not text:
        return 0
    return int(text)


def inspect_sandbox(
    conn: sqlite3.Connection,
    log: FileRememberLog,
    *,
    event_id_path: str | Path | None = None,
) -> RecoverReport:
    """
    Detect unclean state without writing.

    needs_replay if any log event after checkpoint lacks an admitted row,
    or checkpoint lags behind max log id with missing materialize.
    """
    checkpoint = log.load_checkpoint()
    max_log = log.max_event_id()
    alloc_id = _read_allocator_id(
        Path(event_id_path) if event_id_path is not None else None
    )

    pending = 0
    missing = 0
    warnings: list[str] = []

    for event in log.iter_from(checkpoint):
        pending += 1
        if get_admitted(conn, event.event_id) is None:
            missing += 1

    # Also: checkpoint behind max_log always means work may remain
    if max_log > checkpoint and pending == 0:
        # gap or truncated read — still flag
        warnings.append(
            f"max_log_event_id={max_log} > checkpoint={checkpoint} "
            f"but iter_from yielded no events"
        )

    if alloc_id is not None and max_log > 0 and alloc_id < max_log:
        warnings.append(
            f"allocator event_id file ({alloc_id}) < max log event_id ({max_log})"
        )
    if alloc_id is not None and alloc_id > max_log + 1:
        # +1: crashed after next_id before append is possible
        warnings.append(
            f"allocator event_id file ({alloc_id}) ahead of max log ({max_log}); "
            f"possible crash between allocate and append"
        )

    needs_replay = missing > 0 or (max_log > checkpoint and pending > 0)
    # unclean: anything left to replay or warnings about counter/log skew
    unclean = needs_replay or bool(warnings)

    return RecoverReport(
        checkpoint_before=checkpoint,
        max_log_event_id=max_log,
        allocator_event_id=alloc_id,
        pending_event_count=pending,
        missing_admitted_count=missing,
        needs_replay=needs_replay,
        unclean_shutdown_suspected=unclean,
        replayed=0,
        checkpoint_after=checkpoint,
        warnings=tuple(warnings),
    )


def recover_sandbox(
    conn: sqlite3.Connection,
    log: FileRememberLog,
    *,
    admit_lock: threading.Lock | None = None,
    event_id_path: str | Path | None = None,
) -> RecoverReport:
    """
    Startup entry: inspect, then automatic full replay of pending log events.

    Call when opening a sandbox, before serving remember traffic.
    Truth remains SQLite admitted; log is the replay source.
    """
    before = inspect_sandbox(conn, log, event_id_path=event_id_path)
    replayed = 0
    if before.needs_replay or before.pending_event_count > 0:
        built = process_pending_once(conn, log, admit_lock=admit_lock)
        replayed = len(built)
    after = inspect_sandbox(conn, log, event_id_path=event_id_path)
    return RecoverReport(
        checkpoint_before=before.checkpoint_before,
        max_log_event_id=before.max_log_event_id,
        allocator_event_id=before.allocator_event_id,
        pending_event_count=before.pending_event_count,
        missing_admitted_count=before.missing_admitted_count,
        needs_replay=before.needs_replay,
        unclean_shutdown_suspected=before.unclean_shutdown_suspected,
        replayed=replayed,
        checkpoint_after=log.load_checkpoint(),
        warnings=tuple(dict.fromkeys(before.warnings + after.warnings)),
    )


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
    recover_on_start: bool = True,
    event_id_path: str | Path | None = None,
) -> RecoverReport | None:
    """
    Coroutine: optional recover_sandbox once, then poll process_pending_once.
    Returns the startup RecoverReport if recover_on_start else None.
    """
    report: RecoverReport | None = None
    if recover_on_start:
        report = recover_sandbox(
            conn, log, admit_lock=admit_lock, event_id_path=event_id_path
        )
    stop = stop_event if stop_event is not None else asyncio.Event()
    while not stop.is_set():
        process_pending_once(conn, log, admit_lock=admit_lock)
        try:
            await asyncio.wait_for(stop.wait(), timeout=poll_interval)
        except asyncio.TimeoutError:
            continue
    return report


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
