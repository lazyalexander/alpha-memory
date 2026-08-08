"""
ObservationMemory — agent×viewpoint cognition over a node (layer O).

Meaning
-------
What a node is *for a given agent stance*: tags and provenance only.
No reading body, no importance score (scores stay in viewpoint methods later).
Re-interpretation is another Observation (1:N via source_observation_index).

This module holds **data only** (no methods, no ops). Viewpoint as code mem
and act() live elsewhere; here only viewpoint_index is stored.
"""

from __future__ import annotations

from typing import NamedTuple


class ObservationMemory(NamedTuple):
    """Immutable observation metadata record."""

    observation_index: int
    agent_id: int
    viewpoint_index: int
    source_node_index: int
    source_observation_index: int | None
    tags: frozenset[str]
    timestamp: int | float
