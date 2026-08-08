"""
Function contracts for memory processing (Protocol + shape-level helpers).

This module is the interface layer for Viewpoint and Reflection operators.
It does not own memory nodes, pools, or lineage storage.

Memory tools (conceptual only — not implemented here)
----------------------------------------------------
  remember | recall | search | forget | associate | deref

There is **no update** tool. Under pass-only data + append/branch lineage,
effects are only: append a new node / op-log event, or open a new branch
(re-introduce / re-view / re-reflect). In-place mutation is out of scope.

Except introduction, effects on a mem should eventually append to that mem's
op log via Memory tools (tool bodies deferred).
"""

from __future__ import annotations

from typing import Any, Callable, Literal, NamedTuple, Protocol, runtime_checkable

# Re-export-friendly scalar aliases (mirror memory conventions; avoid import cycle)
ImmutableData = Any
ImmutableIndex = int
MemoryTags = frozenset[str]
NumericTimestamp = int | float
FunctionToken = bytes

IndexKind = Literal["data", "view", "reflection", "agent", "agent_action"]
"""agent_action reserved for future tools; no tool body in this module."""

# Reflection graph ops only (abstract lift lives in memory/abstract.py).
ReflectOp = Literal["merge", "forget"]

# Conceptual Memory tools (names only; no implementations).
MEMORY_TOOLS: frozenset[str] = frozenset(
    {"remember", "recall", "search", "forget", "associate", "deref"}
)


class IndexRequest(NamedTuple):
    """
    Request for a Sandbox/Pool-allocated index.

    Viewpoint is bound to index *strategy* (what kind / subject), not to minting
    the final unique number. Pool.allocate is the uniqueness truth source.
    """

    kind: IndexKind
    subject_ref: Any | None = None
    salt: bytes | None = None


class ViewResult(NamedTuple):
    """
    Successful viewpoint application shape (before Pool mints NodeIndex).

    - tags / timestamp: interpretation metadata (timestamp meaningful on view roots)
    - index_request: how to ask the allocator for a node index
    - suggested_node_index: optional pure-path / test hint; Pool must ignore and allocate
    """

    tags: MemoryTags
    timestamp: NumericTimestamp
    index_request: IndexRequest
    suggested_node_index: ImmutableIndex | None = None


@runtime_checkable
class Viewpoint(Protocol):
    """Callable perspective: admitted memory -> ViewResult | None."""

    def __call__(self, admitted: Any) -> ViewResult | None: ...


@runtime_checkable
class AbstractionMethod(Protocol):
    """Concrete reflection body over original (pass-only) data."""

    def __call__(self, data: ImmutableData) -> Any: ...


def ensure_tags(tags: Any) -> MemoryTags:
    """Normalize tags to frozenset[str]."""
    if tags is None:
        return frozenset()
    if isinstance(tags, frozenset):
        return tags
    return frozenset(tags)


def viewpoint_identity(token: FunctionToken, digest: bytes) -> tuple[FunctionToken, bytes]:
    """Stable viewpoint identity key: (token, digest) — not a function name."""
    return (bytes(token), bytes(digest))


def run_viewpoint(fn: Callable[..., Any], admitted: Any) -> ViewResult | None:
    """
    Invoke a viewpoint callable and normalize the result shape.

    Accepts:
    - ViewResult
    - legacy (index, tags, timestamp) used by early memory.apply_viewpoint tests
    - None / exceptions / bad shape -> None
    """
    try:
        raw = fn(admitted)
    except Exception:
        return None
    if raw is None:
        return None
    if isinstance(raw, ViewResult):
        if raw.timestamp is None:
            return None
        return ViewResult(
            tags=ensure_tags(raw.tags),
            timestamp=raw.timestamp,
            index_request=raw.index_request,
            suggested_node_index=raw.suggested_node_index,
        )
    # Legacy triple: (index, tags, timestamp)
    try:
        index, tags, timestamp = raw
    except (TypeError, ValueError):
        return None
    if timestamp is None:
        return None
    if not isinstance(index, int):
        return None
    return ViewResult(
        tags=ensure_tags(tags),
        timestamp=timestamp,
        index_request=IndexRequest(kind="view", subject_ref=index),
        suggested_node_index=index,
    )
