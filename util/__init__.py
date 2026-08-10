"""
Utility implementations (strategies, helpers) — not domain mem records.
"""

from util.event_id import EventIdAllocator, FileEventIdAllocator
from util.index_alloc import (
    STRATEGY_SQLITE_AUTO,
    allocate,
    register_strategy,
    strategy_sqlite_auto,
)

__all__ = [
    "EventIdAllocator",
    "FileEventIdAllocator",
    "STRATEGY_SQLITE_AUTO",
    "allocate",
    "register_strategy",
    "strategy_sqlite_auto",
]
