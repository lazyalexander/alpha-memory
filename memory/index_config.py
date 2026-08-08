"""
Immutable index-strategy configuration shapes (data only).

Per-sandbox: freeze at sandbox creation; do not mutate later.
Maps each mem kind to a strategy *name* (implementation lives in util/).
"""

from __future__ import annotations

from typing import Literal, NamedTuple

MemKind = Literal["admitted", "node", "observation"]

MEM_KINDS: tuple[MemKind, ...] = ("admitted", "node", "observation")


class IndexStrategyConfig(NamedTuple):
    """
    Frozen mapping: mem kind -> strategy name.

    Strategy names are resolved by util.index_alloc (e.g. \"sqlite_auto\").
    """

    admitted: str
    node: str
    observation: str


# Default: all kinds defer primary-key generation to SQLite.
DEFAULT_SQLITE_AUTO = IndexStrategyConfig(
    admitted="sqlite_auto",
    node="sqlite_auto",
    observation="sqlite_auto",
)
