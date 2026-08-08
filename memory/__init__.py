"""
memory package — first-class memory kinds for the multi-agent substrate.

Layout convention (keep forever)
--------------------------------
1. **One memory kind → one module** under this package.
2. **Core data type(s) + functions for that kind live in the same file.**
3. **Viewpoint is special:** it is required for original admission, so
   `ViewpointMemory` lives with Original in `original.py` (not a separate
   orphan module, not under abstract/reflect).
4. Shared scalars / FunctionMark only → `common.py` (not a memory kind).
5. Runtime registry → `pool.py` (not a memory kind).

Modules
-------
- common.py    shared aliases, FunctionMark
- original.py  ViewpointMemory + AdmittedMemory + admit / bootstrap
- abstract.py  AbstractMemory + view / lift / reabstract
- reflect.py   ReflectMemory + merge / forget
- pool.py      MemoryPool

External contracts for callables remain in top-level `func.py`.
"""

from memory.abstract import (
    AbstractCoreInfo,
    AbstractMemory,
    AbstractionDeclarator,
    abstract_memory,
    apply_viewpoint,
    branch_root,
    core_info,
    is_stale,
    reabstract,
    root_of,
    same_source,
    walk_origin,
)
from memory.common import (
    Annotator,
    DataValidator,
    FunctionMark,
    FunctionToken,
    ImmutableAnnotation,
    ImmutableData,
    ImmutableIndex,
    IndexValidator,
    MemoryTags,
    NumericTimestamp,
    SandboxId,
)
from memory.original import (
    AdmittedMemory,
    ViewpointFn,
    ViewpointMemory,
    admit_memory,
    bootstrap_viewpoint,
    viewpoint_matches,
)
from memory.pool import AgentRecord, MemoryPool, OpEvent
from memory.reflect import (
    ReflectMemory,
    ReflectOp,
    StructuralNode,
    reflect_forget,
    reflect_merge,
    walk_origins,
)

__all__ = [
    "AbstractCoreInfo",
    "AbstractMemory",
    "AbstractionDeclarator",
    "AdmittedMemory",
    "AgentRecord",
    "Annotator",
    "DataValidator",
    "FunctionMark",
    "FunctionToken",
    "ImmutableAnnotation",
    "ImmutableData",
    "ImmutableIndex",
    "IndexValidator",
    "MemoryPool",
    "MemoryTags",
    "NumericTimestamp",
    "OpEvent",
    "ReflectMemory",
    "ReflectOp",
    "SandboxId",
    "StructuralNode",
    "ViewpointFn",
    "ViewpointMemory",
    "abstract_memory",
    "admit_memory",
    "apply_viewpoint",
    "bootstrap_viewpoint",
    "branch_root",
    "core_info",
    "is_stale",
    "reabstract",
    "reflect_forget",
    "reflect_merge",
    "root_of",
    "same_source",
    "viewpoint_matches",
    "walk_origin",
    "walk_origins",
]
