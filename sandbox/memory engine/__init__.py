"""
Memory Engine — I/O, persistence cost, cache, and retrieval acceleration.

Responsibility
--------------
Owns physical access paths for A/N/O: SQLite (or future backends), append
paths, secondary index/cache tables, activation of hot sets, and query
acceleration over immutable primary rows.

Does not decide agent policy or act semantics — that is Memory Stream.
Domain record types remain in memory/ (data only).

Import note
-----------
This directory name contains a space; load ``engine`` by path or::

    import importlib.util
    from pathlib import Path
    p = Path(__file__).resolve().parent / "engine.py"
    ...
"""

from __future__ import annotations

from pathlib import Path

# Re-export build API when package is loaded via path hacks / importlib.
# Standard ``import sandbox...`` fails because of the space in the directory name.

def _load_engine():
    import importlib.util

    path = Path(__file__).resolve().parent / "engine.py"
    spec = importlib.util.spec_from_file_location(
        "alpha_memory_engine", path
    )
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load engine from {path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_engine = _load_engine()
build_admitted = _engine.build_admitted
build_node = _engine.build_node
build_observation = _engine.build_observation
IndexAcquire = _engine.IndexAcquire

__all__ = [
    "IndexAcquire",
    "build_admitted",
    "build_node",
    "build_observation",
]
