"""
SQLite persistence for AdmittedMemory / NodeMemory / ObservationMemory.

Free functions only — not methods on memory records.
"""

from __future__ import annotations

import json
import sqlite3
from pathlib import Path

from memory import AdmittedMemory, NodeMemory, ObservationMemory

_SCHEMA_PATH = Path(__file__).with_name("schema.sql")


def connect(db_path: str | Path) -> sqlite3.Connection:
    """Open connection with foreign keys on. Creates parent dirs if needed."""
    path = Path(db_path)
    if path.parent != Path(".") and str(path.parent) != "":
        path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(path))
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    return conn


def init_db(conn: sqlite3.Connection) -> None:
    """Apply schema.sql."""
    sql = _SCHEMA_PATH.read_text(encoding="utf-8")
    conn.executescript(sql)
    conn.commit()


def insert_admitted(conn: sqlite3.Connection, row: AdmittedMemory) -> None:
    conn.execute(
        "INSERT INTO admitted (admitted_index, data, from_agent) VALUES (?, ?, ?)",
        (row.admitted_index, row.data, row.from_agent),
    )
    conn.commit()


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


def insert_node(conn: sqlite3.Connection, row: NodeMemory) -> None:
    conn.execute(
        """
        INSERT INTO node (
            node_index, sandbox_id, timestamp, parent_node_index, admitted_index
        ) VALUES (?, ?, ?, ?, ?)
        """,
        (
            row.node_index,
            row.sandbox_id,
            float(row.timestamp),
            row.parent_node_index,
            row.admitted_index,
        ),
    )
    conn.commit()


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


def insert_observation(conn: sqlite3.Connection, row: ObservationMemory) -> None:
    conn.execute(
        """
        INSERT INTO observation (
            observation_index, agent_id, viewpoint_index,
            source_node_index, source_observation_index, tags_json, timestamp
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        """,
        (
            row.observation_index,
            row.agent_id,
            row.viewpoint_index,
            row.source_node_index,
            row.source_observation_index,
            _tags_to_json(row.tags),
            float(row.timestamp),
        ),
    )
    conn.commit()


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
