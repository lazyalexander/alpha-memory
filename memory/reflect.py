"""
reflect.py — ReflectMemory (graph-level merge / forget).

Convention: core data + functions for this memory kind live only here.

Meaning
-------
**Reflect** operates on the structural node graph — not single-chain semantic lift.

Ops: merge (M1, multi origin / multi data_index), forget (tombstone).
Abstract lift is not Reflect (see abstract.py).
No update: only append ReflectMemory nodes.
"""

from __future__ import annotations

from typing import Iterator, Literal, NamedTuple, Sequence, Union

from func import ensure_tags

from memory.abstract import AbstractMemory
from memory.common import ImmutableIndex, MemoryTags
from memory.original import AdmittedMemory, ViewpointMemory

ReflectOp = Literal["merge", "forget"]
StructuralNode = Union[AbstractMemory, "ReflectMemory"]


class ReflectMemory(NamedTuple):
    """Core data: one reflection node on the structural graph."""

    op: ReflectOp
    source: AdmittedMemory
    viewpoint: ViewpointMemory
    index: ImmutableIndex
    tags: MemoryTags
    origins: tuple[StructuralNode, ...]
    data_indexes: tuple[ImmutableIndex, ...]
    timestamp: None = None

    @property
    def node_index(self) -> ImmutableIndex:
        return self.index

    @property
    def reflection_op(self) -> ReflectOp:
        return self.op

    @property
    def origin(self) -> StructuralNode | None:
        if self.op == "forget" and len(self.origins) == 1:
            return self.origins[0]
        return None


def _data_indexes_of(node: StructuralNode) -> tuple[ImmutableIndex, ...]:
    if node.data_indexes:
        return node.data_indexes
    return (node.source.data_index,)


def _is_structural(node: object) -> bool:
    return isinstance(node, (AbstractMemory, ReflectMemory))


def reflect_merge(
    priors: Sequence[StructuralNode],
    *,
    index: ImmutableIndex,
    viewpoint: ViewpointMemory,
    tags: MemoryTags | None = None,
) -> ReflectMemory | None:
    """M1 merge: multiple origins and data_indexes."""
    if not priors or len(priors) < 2:
        return None
    if not all(_is_structural(p) for p in priors):
        return None
    if not isinstance(viewpoint, ViewpointMemory):
        return None
    seen: list[ImmutableIndex] = []
    for p in priors:
        for di in _data_indexes_of(p):
            if di not in seen:
                seen.append(di)
    if not seen:
        return None
    return ReflectMemory(
        op="merge",
        source=priors[0].source,
        viewpoint=viewpoint,
        index=index,
        tags=ensure_tags(tags),
        origins=tuple(priors),
        data_indexes=tuple(seen),
        timestamp=None,
    )


def reflect_forget(
    target: StructuralNode,
    *,
    index: ImmutableIndex,
    viewpoint: ViewpointMemory,
    tags: MemoryTags | None = None,
) -> ReflectMemory | None:
    """Tombstone node; does not delete or mutate blob."""
    if not _is_structural(target):
        return None
    if not isinstance(viewpoint, ViewpointMemory):
        return None
    return ReflectMemory(
        op="forget",
        source=target.source,
        viewpoint=viewpoint,
        index=index,
        tags=ensure_tags(tags),
        origins=(target,),
        data_indexes=_data_indexes_of(target),
        timestamp=None,
    )


def walk_origins(node: StructuralNode) -> Iterator[StructuralNode]:
    """DFS over origins (cycle-safe)."""
    stack: list[StructuralNode] = [node]
    seen: set[int] = set()
    while stack:
        current = stack.pop()
        cid = id(current)
        if cid in seen:
            continue
        seen.add(cid)
        yield current
        for parent in reversed(current.origins):
            if id(parent) not in seen:
                stack.append(parent)
