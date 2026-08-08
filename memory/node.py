"""
NodeMemory — git-style collab node over admitted data (layer N).

Meaning
-------
Versioned pointer into AdmittedMemory for multi-agent collaboration.
Modifications create a new node (parent link), never mutate an existing one.
Conceptually invisible to agents; used by pool/storage/retrieval later.

This module holds **data only** (no methods, no ops). Git tree maintenance
and branching logic live elsewhere. Never embed payload bytes — only
``admitted_index``.
"""

from __future__ import annotations

from typing import NamedTuple


class NodeMemory(NamedTuple):
    """Immutable node record; payload by pointer only."""

    node_index: int
    sandbox_id: str
    timestamp: int | float
    parent_node_index: int | None
    admitted_index: int
