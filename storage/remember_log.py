"""
Durable remember-event log (append-only intents).

event_id is assigned by util.EventIdAllocator (not SQLite autoincrement).
status: pending | done | failed
Materialize success => admitted row with admitted_index = event_id exists.
"""

from __future__ import annotations

import sqlite3
from typing import NamedTuple


class RememberEvent(NamedTuple):
    event_id: int
    data: str
    from_agent: int
    status: str
    error: str | None


def append_pending(
    conn: sqlite3.Connection,
    *,
    event_id: int,
    data: str,
    from_agent: int,
    commit: bool = True,
) -> None:
    conn.execute(
        """
        INSERT INTO remember_event (event_id, data, from_agent, status, error)
        VALUES (?, ?, ?, 'pending', NULL)
        """,
        (event_id, data, from_agent),
    )
    if commit:
        conn.commit()


def list_pending(conn: sqlite3.Connection) -> list[RememberEvent]:
    cur = conn.execute(
        """
        SELECT event_id, data, from_agent, status, error
        FROM remember_event
        WHERE status = 'pending'
        ORDER BY event_id ASC
        """
    )
    return [
        RememberEvent(
            event_id=int(r[0]),
            data=str(r[1]),
            from_agent=int(r[2]),
            status=str(r[3]),
            error=None if r[4] is None else str(r[4]),
        )
        for r in cur.fetchall()
    ]


def mark_done(conn: sqlite3.Connection, event_id: int, *, commit: bool = True) -> None:
    conn.execute(
        "UPDATE remember_event SET status = 'done', error = NULL WHERE event_id = ?",
        (event_id,),
    )
    if commit:
        conn.commit()


def mark_failed(
    conn: sqlite3.Connection,
    event_id: int,
    error: str,
    *,
    commit: bool = True,
) -> None:
    conn.execute(
        "UPDATE remember_event SET status = 'failed', error = ? WHERE event_id = ?",
        (error, event_id),
    )
    if commit:
        conn.commit()


def get_event(conn: sqlite3.Connection, event_id: int) -> RememberEvent | None:
    cur = conn.execute(
        """
        SELECT event_id, data, from_agent, status, error
        FROM remember_event WHERE event_id = ?
        """,
        (event_id,),
    )
    r = cur.fetchone()
    if r is None:
        return None
    return RememberEvent(
        event_id=int(r[0]),
        data=str(r[1]),
        from_agent=int(r[2]),
        status=str(r[3]),
        error=None if r[4] is None else str(r[4]),
    )
