"""
Pluggable index allocation by strategy name.

allocate(config, kind) -> int | None
  None  — defer to SQLite (omit PK on INSERT; use lastrowid)
  int   — use this value as the primary key

Sandbox freezes IndexStrategyConfig at creation; this module only resolves
strategy names to callables.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import TypeAlias

from memory.index_config import IndexStrategyConfig, MemKind

IndexFn: TypeAlias = Callable[[], int | None]

STRATEGY_SQLITE_AUTO = "sqlite_auto"


def strategy_sqlite_auto() -> int | None:
    """
    Hand primary-key generation to SQLite.

    Returns None so storage INSERTs omit the id column and read lastrowid.
    """
    return None


_REGISTRY: dict[str, IndexFn] = {
    STRATEGY_SQLITE_AUTO: strategy_sqlite_auto,
}


def register_strategy(name: str, fn: IndexFn) -> None:
    """Register or replace a named strategy (tests / future backends)."""
    if not name:
        raise ValueError("strategy name must be non-empty")
    _REGISTRY[name] = fn


def allocate(config: IndexStrategyConfig, kind: MemKind) -> int | None:
    """
    Resolve sandbox-frozen config for ``kind`` and run that strategy.

    Returns int to use as PK, or None to let SQLite assign.
    """
    if kind == "admitted":
        name = config.admitted
    elif kind == "node":
        name = config.node
    elif kind == "observation":
        name = config.observation
    else:
        raise ValueError(f"unknown mem kind: {kind!r}")

    try:
        fn = _REGISTRY[name]
    except KeyError as e:
        raise KeyError(f"unknown index strategy: {name!r}") from e
    result = fn()
    if result is not None and (not isinstance(result, int) or isinstance(result, bool)):
        raise TypeError(f"strategy {name!r} must return int | None, got {type(result)}")
    if isinstance(result, int) and result < 0:
        raise ValueError(f"strategy {name!r} returned negative index: {result}")
    return result
