"""
Persistence for A/N/O memory records.

Domain types stay in ``memory/`` (data only). SQL + connection ops live here.
"""

from storage.remember_log import (
    RememberEvent,
    append_pending,
    get_event,
    list_pending,
    mark_done,
    mark_failed,
)
from storage.sqlite_store import (
    connect,
    get_admitted,
    get_node,
    get_observation,
    init_db,
    insert_admitted,
    insert_node,
    insert_observation,
)

__all__ = [
    "RememberEvent",
    "append_pending",
    "connect",
    "get_admitted",
    "get_event",
    "get_node",
    "get_observation",
    "init_db",
    "insert_admitted",
    "insert_node",
    "insert_observation",
    "list_pending",
    "mark_done",
    "mark_failed",
]
