"""
SQLite persistence for AdmittedMemory / NodeMemory / ObservationMemory.

Free functions only — not methods on memory records.

Index policy:
  admitted_index / node_index / observation_index may be passed as int, or
  None to omit the PK column and let SQLite assign (then lastrowid is used).
"""

from __future__ import annotations

import json
import sqlite3
from pathlib import Path

from memory import AdmittedMemory, NodeMemory, ObservationMemory

_SCHEMA_PATH = Path(__file__).with_name("schema.sql")


def connect(
    db_path: str | Path,
    *,
    check_same_thread: bool = True,
) -> sqlite3.Connection:
    """
    Open connection with foreign keys on. Creates parent dirs if needed.

    check_same_thread=False: allow one connection shared across threads when
    callers serialize writes (e.g. sandbox admit_lock on build_admitted).
    """
    path = Path(db_path)
    if path.parent != Path(".") and str(path.parent) != "":
        path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(path), check_same_thread=check_same_thread)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    return conn


def init_db(conn: sqlite3.Connection) -> None:
    """Apply schema.sql."""
    sql = _SCHEMA_PATH.read_text(encoding="utf-8")
    conn.executescript(sql)
    conn.commit()


def insert_admitted(
    conn: sqlite3.Connection,
    *,
    data: str,
    from_agent: int,
    admitted_index: int | None = None,
    commit: bool = True,
) -> AdmittedMemory:
    """
    Insert admitted row. If admitted_index is None, SQLite assigns the PK.
    Returns a complete AdmittedMemory with the final index.

    commit=False: leave the transaction open for the caller (e.g. BEGIN IMMEDIATE).
    """
    if admitted_index is None:
        cur = conn.execute(
            "INSERT INTO admitted (data, from_agent) VALUES (?, ?)",
            (data, from_agent),
        )
        new_id = int(cur.lastrowid)
    else:
        cur = conn.execute(
            "INSERT INTO admitted (admitted_index, data, from_agent) VALUES (?, ?, ?)",
            (admitted_index, data, from_agent),
        )
        new_id = admitted_index
    if commit:
        conn.commit()
    return AdmittedMemory(
        admitted_index=new_id,
        data=data,
        from_agent=from_agent,
    )


def get_admitted(
    conn: sqlite3.Connection, admitted_index: int
) -> AdmittedMemory | None:
    cur = conn.execute(
        "SELECT admitted_index, data, from_agent FROM admitted WHERE admitted_index = ?",
        (admitted_index,),
    )
    r = cur.fetchone()
    if r is None:
        return None
    return AdmittedMemory(
        admitted_index=int(r["admitted_index"]),
        data=str(r["data"]),
        from_agent=int(r["from_agent"]),
    )


def insert_node(
    conn: sqlite3.Connection,
    *,
    sandbox_id: str,
    timestamp: int | float,
    parent_node_index: int | None,
    admitted_index: int,
    node_index: int | None = None,
) -> NodeMemory:
    """Insert node; node_index None => SQLite assigns PK."""
    if node_index is None:
        cur = conn.execute(
            """
            INSERT INTO node (
                sandbox_id, timestamp, parent_node_index, admitted_index
            ) VALUES (?, ?, ?, ?)
            """,
            (sandbox_id, float(timestamp), parent_node_index, admitted_index),
        )
        new_id = int(cur.lastrowid)
    else:
        cur = conn.execute(
            """
            INSERT INTO node (
                node_index, sandbox_id, timestamp, parent_node_index, admitted_index
            ) VALUES (?, ?, ?, ?, ?)
            """,
            (
                node_index,
                sandbox_id,
                float(timestamp),
                parent_node_index,
                admitted_index,
            ),
        )
        new_id = node_index
    conn.commit()
    return NodeMemory(
        node_index=new_id,
        sandbox_id=sandbox_id,
        timestamp=float(timestamp),
        parent_node_index=parent_node_index,
        admitted_index=admitted_index,
    )


def get_node(conn: sqlite3.Connection, node_index: int) -> NodeMemory | None:
    cur = conn.execute(
        """
        SELECT node_index, sandbox_id, timestamp, parent_node_index, admitted_index
        FROM node WHERE node_index = ?
        """,
        (node_index,),
    )
    r = cur.fetchone()
    if r is None:
        return None
    parent = r["parent_node_index"]
    return NodeMemory(
        node_index=int(r["node_index"]),
        sandbox_id=str(r["sandbox_id"]),
        timestamp=float(r["timestamp"]),
        parent_node_index=None if parent is None else int(parent),
        admitted_index=int(r["admitted_index"]),
    )


def _tags_to_json(tags: frozenset[str]) -> str:
    return json.dumps(sorted(tags), ensure_ascii=False)


def _tags_from_json(raw: str) -> frozenset[str]:
    data = json.loads(raw)
    if not isinstance(data, list):
        raise ValueError("tags_json must be a JSON array")
    return frozenset(str(x) for x in data)


def insert_observation(
    conn: sqlite3.Connection,
    *,
    agent_id: int,
    viewpoint_index: int,
    source_node_index: int,
    source_observation_index: int | None,
    tags: frozenset[str],
    timestamp: int | float,
    observation_index: int | None = None,
) -> ObservationMemory:
    """Insert observation; observation_index None => SQLite assigns PK."""
    tags_json = _tags_to_json(tags)
    if observation_index is None:
        cur = conn.execute(
            """
            INSERT INTO observation (
                agent_id, viewpoint_index,
                source_node_index, source_observation_index, tags_json, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?)
            """,
            (
                agent_id,
                viewpoint_index,
                source_node_index,
                source_observation_index,
                tags_json,
                float(timestamp),
            ),
        )
        new_id = int(cur.lastrowid)
    else:
        cur = conn.execute(
            """
            INSERT INTO observation (
                observation_index, agent_id, viewpoint_index,
                source_node_index, source_observation_index, tags_json, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            (
                observation_index,
                agent_id,
                viewpoint_index,
                source_node_index,
                source_observation_index,
                tags_json,
                float(timestamp),
            ),
        )
        new_id = observation_index
    conn.commit()
    return ObservationMemory(
        observation_index=new_id,
        agent_id=agent_id,
        viewpoint_index=viewpoint_index,
        source_node_index=source_node_index,
        source_observation_index=source_observation_index,
        tags=tags,
        timestamp=float(timestamp),
    )


def get_observation(
    conn: sqlite3.Connection, observation_index: int
) -> ObservationMemory | None:
    cur = conn.execute(
        """
        SELECT observation_index, agent_id, viewpoint_index,
               source_node_index, source_observation_index, tags_json, timestamp
        FROM observation WHERE observation_index = ?
        """,
        (observation_index,),
    )
    r = cur.fetchone()
    if r is None:
        return None
    parent_obs = r["source_observation_index"]
    return ObservationMemory(
        observation_index=int(r["observation_index"]),
        agent_id=int(r["agent_id"]),
        viewpoint_index=int(r["viewpoint_index"]),
        source_node_index=int(r["source_node_index"]),
        source_observation_index=None if parent_obs is None else int(parent_obs),
        tags=_tags_from_json(str(r["tags_json"])),
        timestamp=float(r["timestamp"]),
    )
