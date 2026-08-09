"""
Memory Engine core I/O façade — build complete A/N/O records.

Responsibility
--------------
- Append-only insert of Admitted / Node / Observation via storage/.
- Build = acquire index + persist + return full domain record.
- Visible mem is always post-insert (stable *_index + row exists).

No updates or deletes of A/N/O domain rows. Branching = new Node rows.

Index acquisition
-----------------
``acquire_index`` is a zero-arg callable: () -> int | None
  None — SQLite assigns primary key (lastrowid)
  int  — use that primary key

Typical binding (sandbox-frozen config)::

    from functools import partial
    from util.index_alloc import allocate
    acquire = partial(allocate, sandbox_index_config, "admitted")
    build_admitted(conn, data=..., from_agent=..., acquire_index=acquire)
"""

from __future__ import annotations

import sqlite3
from collections.abc import Callable
from typing import TypeAlias

from memory import AdmittedMemory, NodeMemory, ObservationMemory
from storage.sqlite_store import (
    get_admitted,
    get_node,
    get_observation,
    insert_admitted,
    insert_node,
    insert_observation,
)

# () -> int | None  (None => defer PK to SQLite)
IndexAcquire: TypeAlias = Callable[[], int | None]


def _resolve_index(acquire_index: IndexAcquire) -> int | None:
    idx = acquire_index()
    if idx is not None and (not isinstance(idx, int) or isinstance(idx, bool)):
        raise TypeError(
            f"acquire_index must return int | None, got {type(idx).__name__}"
        )
    if isinstance(idx, int) and idx < 0:
        raise ValueError(f"acquire_index returned negative index: {idx}")
    return idx


def build_admitted(
    conn: sqlite3.Connection,
    *,
    data: str,
    from_agent: int,
    acquire_index: IndexAcquire,
) -> AdmittedMemory:
    """
    Build and persist AdmittedMemory (single payload ingress).

    acquire_index: index method for admitted kind.
    Returns complete record only after successful INSERT.
    """
    if not isinstance(data, str):
        raise TypeError("data must be str (MVP)")
    if data == "":
        raise ValueError("data must be non-empty")
    idx = _resolve_index(acquire_index)
    return insert_admitted(
        conn,
        data=data,
        from_agent=from_agent,
        admitted_index=idx,
    )


def build_node(
    conn: sqlite3.Connection,
    *,
    sandbox_id: str,
    timestamp: int | float,
    parent_node_index: int | None,
    admitted_index: int,
    acquire_index: IndexAcquire,
) -> NodeMemory:
    """
    Build and persist NodeMemory pointing at an existing Admitted row.

    admitted_index must already exist (FK). parent_node_index if set must exist.
    Returns complete record only after successful INSERT.
    """
    if not sandbox_id:
        raise ValueError("sandbox_id must be non-empty")
    if get_admitted(conn, admitted_index) is None:
        raise ValueError(
            f"admitted_index={admitted_index} not found; admit before node"
        )
    if parent_node_index is not None and get_node(conn, parent_node_index) is None:
        raise ValueError(
            f"parent_node_index={parent_node_index} not found"
        )
    idx = _resolve_index(acquire_index)
    return insert_node(
        conn,
        sandbox_id=sandbox_id,
        timestamp=timestamp,
        parent_node_index=parent_node_index,
        admitted_index=admitted_index,
        node_index=idx,
    )


def build_observation(
    conn: sqlite3.Connection,
    *,
    agent_id: int,
    viewpoint_index: int,
    source_node_index: int,
    source_observation_index: int | None,
    tags: frozenset[str],
    timestamp: int | float,
    acquire_index: IndexAcquire,
) -> ObservationMemory:
    """
    Build and persist ObservationMemory (single parent: node and/or observation).

    source_node_index must exist. If source_observation_index is set, that
    observation must exist (re-observation chain).
    Returns complete record only after successful INSERT.
    """
    if get_node(conn, source_node_index) is None:
        raise ValueError(
            f"source_node_index={source_node_index} not found; "
            "node must be persisted before observation"
        )
    if source_observation_index is not None:
        if get_observation(conn, source_observation_index) is None:
            raise ValueError(
                f"source_observation_index={source_observation_index} not found"
            )
    if not isinstance(tags, frozenset):
        tags = frozenset(tags)
    idx = _resolve_index(acquire_index)
    return insert_observation(
        conn,
        agent_id=agent_id,
        viewpoint_index=viewpoint_index,
        source_node_index=source_node_index,
        source_observation_index=source_observation_index,
        tags=tags,
        timestamp=timestamp,
        observation_index=idx,
    )
