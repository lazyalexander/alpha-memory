"""
abstract.py — AbstractMemory (single-parent lift).

Convention: core data + functions for this memory kind live only here.

Meaning
-------
**Abstract** is semantic elevation along one line of interpretation.

- re-describes the same original blob (share; no mutate);
- at most one parent;
- no merge / no tombstone;
- never rewrites introduction provenance.

Paths
-----
1. View root — apply_viewpoint(admitted); origins=(); timestamp set.
2. Lift — abstract_memory(prior, declare, ...); origins=(prior,); timestamp=None.
3. reabstract — new branch from raw data (No-Update).
"""

from __future__ import annotations

from typing import Callable, Iterator, NamedTuple

from func import AbstractionMethod, ensure_tags, run_viewpoint

from memory.common import (
    Annotator,
    DataValidator,
    FunctionToken,
    ImmutableData,
    ImmutableIndex,
    IndexValidator,
    MemoryTags,
    NumericTimestamp,
    SandboxId,
)
from memory.original import (
    AdmittedMemory,
    ViewpointMemory,
    admit_memory,
    viewpoint_matches,
)


class AbstractCoreInfo(NamedTuple):
    """Pass-only slice of an AbstractMemory without lineage pointers."""

    viewpoint_token: FunctionToken
    viewpoint_digest: bytes
    index: ImmutableIndex
    tags: MemoryTags
    timestamp: NumericTimestamp | None


class AbstractMemory(NamedTuple):
    """
    Core data: one abstract (or view-root) node on a single-parent chain.
    """

    source: AdmittedMemory
    viewpoint: ViewpointMemory
    index: ImmutableIndex
    tags: MemoryTags
    timestamp: NumericTimestamp | None
    origins: tuple[AbstractMemory, ...] = ()
    data_indexes: tuple[ImmutableIndex, ...] = ()

    @property
    def origin(self) -> AbstractMemory | None:
        if len(self.origins) == 1:
            return self.origins[0]
        return None

    @property
    def node_index(self) -> ImmutableIndex:
        return self.index

    @property
    def is_view_root(self) -> bool:
        return len(self.origins) == 0


def core_info(node: AbstractMemory) -> AbstractCoreInfo:
    return AbstractCoreInfo(
        viewpoint_token=node.viewpoint.mark.token,
        viewpoint_digest=node.viewpoint.digest,
        index=node.index,
        tags=node.tags,
        timestamp=node.timestamp,
    )


def _data_indexes_of(node: AbstractMemory) -> tuple[ImmutableIndex, ...]:
    if node.data_indexes:
        return node.data_indexes
    return (node.source.data_index,)


def apply_viewpoint(
    admitted: AdmittedMemory,
    *,
    node_index: ImmutableIndex | None = None,
) -> AbstractMemory | None:
    """First abstract layer: view-root under embedded Viewpoint."""
    if not isinstance(admitted, AdmittedMemory):
        return None
    vr = run_viewpoint(admitted.viewpoint.mark.fn, admitted)
    if vr is None:
        return None
    idx = node_index if node_index is not None else vr.suggested_node_index
    if idx is None:
        return None
    return AbstractMemory(
        source=admitted,
        viewpoint=admitted.viewpoint,
        index=idx,
        tags=ensure_tags(vr.tags),
        timestamp=vr.timestamp,
        origins=(),
        data_indexes=(admitted.data_index,),
    )


DeclareResult = tuple[AbstractionMethod, ViewpointMemory]
AbstractionDeclarator = Callable[
    [AbstractCoreInfo, MemoryTags, ViewpointMemory],
    DeclareResult | None,
]


def abstract_memory(
    prior: AbstractMemory,
    declare: AbstractionDeclarator,
    *,
    index: ImmutableIndex,
    tags: MemoryTags | None = None,
) -> AbstractMemory | None:
    """One-parent abstract lift over a prior AbstractMemory."""
    if not isinstance(prior, AbstractMemory):
        return None
    if len(prior.origins) > 1:
        return None
    info = core_info(prior)
    try:
        declared = declare(info, prior.tags, prior.viewpoint)
    except Exception:
        return None
    if declared is None:
        return None
    try:
        fc, abstractor = declared
    except (TypeError, ValueError):
        return None
    if not isinstance(abstractor, ViewpointMemory):
        return None
    if not callable(fc):
        return None
    try:
        fc(prior.source.data)
    except Exception:
        return None
    return AbstractMemory(
        source=prior.source,
        viewpoint=abstractor,
        index=index,
        tags=ensure_tags(tags),
        timestamp=None,
        origins=(prior,),
        data_indexes=_data_indexes_of(prior),
    )


def reabstract(
    data: ImmutableData,
    viewpoint: ViewpointMemory,
    data_validator: DataValidator,
    annotator: Annotator,
    admit_index: ImmutableIndex,
    index_validator: IndexValidator,
    *,
    sandbox_id: SandboxId = "",
    introduced_by: ImmutableIndex = -1,
    introduced_via: str = "reabstract",
) -> AbstractMemory | None:
    """New abstract branch from raw data (No-Update)."""
    admitted = admit_memory(
        data,
        viewpoint,
        data_validator,
        annotator,
        admit_index,
        index_validator,
        sandbox_id=sandbox_id,
        introduced_by=introduced_by,
        introduced_via=introduced_via,
    )
    if admitted is None:
        return None
    return apply_viewpoint(admitted)


def is_stale(node: AbstractMemory, current: ViewpointMemory) -> bool:
    return not viewpoint_matches(node.viewpoint, current)


def walk_origin(node: AbstractMemory) -> Iterator[AbstractMemory]:
    current: AbstractMemory | None = node
    seen: set[int] = set()
    while current is not None and id(current) not in seen:
        seen.add(id(current))
        yield current
        current = current.origin


def root_of(node: AbstractMemory) -> AbstractMemory:
    current = node
    guard = 0
    while current.origin is not None and guard < 10_000:
        current = current.origin
        guard += 1
    return current


def same_source(a: AbstractMemory, b: AbstractMemory) -> bool:
    return a.source is b.source


branch_root = apply_viewpoint
