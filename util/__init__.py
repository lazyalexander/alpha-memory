"""
Utility implementations (strategies, helpers) — not domain mem records.
"""

from util.index_alloc import (
    STRATEGY_SQLITE_AUTO,
    allocate,
    register_strategy,
    strategy_sqlite_auto,
)

__all__ = [
    "STRATEGY_SQLITE_AUTO",
    "allocate",
    "register_strategy",
    "strategy_sqlite_auto",
]
