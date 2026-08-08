"""
memory package — first-class memory records (data layer only).

Layout convention
-----------------
1. **One memory kind → one file** under this package.
2. That file holds **data only**: immutable records, **no methods**, no ops.
3. Construction, Pool, Agent, Viewpoint code-mem, and act/search live in
   **other modules** (not co-located as “struct + methods”).
4. Shared aliases only → ``common.py`` (not a memory kind).

Kinds
-----
- admitted.py      AdmittedMemory   (legal payload, str MVP)
- node.py          NodeMemory       (git-style pointer to admitted)
- observation.py   ObservationMemory (agent×viewpoint cognition metadata)

Index strategy config (data only): index_config.py
  Implementations of allocate live in util/index_alloc.py.
"""

from memory.admitted import AdmittedMemory
from memory.common import (
    ImmutableIndex,
    MemoryTags,
    NumericTimestamp,
    SandboxId,
)
from memory.index_config import (
    DEFAULT_SQLITE_AUTO,
    MEM_KINDS,
    IndexStrategyConfig,
    MemKind,
)
from memory.node import NodeMemory
from memory.observation import ObservationMemory

__all__ = [
    "DEFAULT_SQLITE_AUTO",
    "AdmittedMemory",
    "ImmutableIndex",
    "IndexStrategyConfig",
    "MEM_KINDS",
    "MemKind",
    "MemoryTags",
    "NodeMemory",
    "NumericTimestamp",
    "ObservationMemory",
    "SandboxId",
]
