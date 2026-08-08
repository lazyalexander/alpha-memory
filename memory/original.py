"""
original.py — Original (admitted) memory + Viewpoint memory.

Convention
----------
One memory kind per module: core data types + functions co-located.

**Viewpoint is special:** it is first-class memory and is the admission gate
for originals. Therefore ViewpointMemory lives **here**, with Original /
AdmittedMemory — not in a separate kind folder and not in abstract/reflect.

Kinds in this module
--------------------
1. ViewpointMemory — interpretation perspective (function mark + digest + …)
2. AdmittedMemory  — original payload after admit under a Viewpoint

Axioms
------
- Original data is pass-only; no in-place update.
- admit_memory requires a ViewpointMemory (no naked data).
- bootstrap_viewpoint is the only genesis path for viewpoints without a prior
  ViewpointMemory.
- Introduction provenance is set at admit/introduce only; Abstract/Reflect
  never rewrite it.
"""

from __future__ import annotations

from typing import Callable, NamedTuple

from func import ViewResult

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


class ViewpointMemory(NamedTuple):
    """
    Viewpoint-as-memory: an interpretation perspective.

    Pass-only / immutable. Identity for allowlist and staleness:
    (mark.token, digest) — not __name__.
    Required to admit original data.
    """

    mark: FunctionMark
    digest: bytes
    index: ImmutableIndex
    tags: MemoryTags
    timestamp: NumericTimestamp
    meta: ImmutableAnnotation


ViewpointFn = Callable[["AdmittedMemory"], ViewResult | tuple | None]


def bootstrap_viewpoint(
    fn: ViewpointFn,
    token: FunctionToken,
    digest: bytes,
    index: ImmutableIndex,
    tags: MemoryTags | None = None,
    timestamp: NumericTimestamp = 0,
    meta: ImmutableAnnotation = "",
) -> ViewpointMemory:
    """
    Genesis path: create ViewpointMemory without a prior viewpoint.

    Only admission-free constructor for viewpoints.
    """
    if not isinstance(token, (bytes, bytearray)):
        raise TypeError("token must be bytes")
    if not isinstance(digest, (bytes, bytearray)):
        raise TypeError("digest must be bytes")
    return ViewpointMemory(
        mark=FunctionMark(fn=fn, token=bytes(token)),
        digest=bytes(digest),
        index=index,
        tags=frozenset() if tags is None else frozenset(tags),
        timestamp=timestamp,
        meta=meta,
    )


def viewpoint_matches(a: ViewpointMemory, b: ViewpointMemory) -> bool:
    """True iff viewpoint identity matches: (token, digest)."""
    return a.mark.token == b.mark.token and a.digest == b.digest


class AdmittedMemory(NamedTuple):
    """
    Original memory after admission under a required ViewpointMemory.

    data is shared by all abstract/reflect branches (pass-only).
    Introduction provenance is immutable after create:
      sandbox_id, data_index, introduced_by, introduced_via.
    """

    data: ImmutableData
    viewpoint: ViewpointMemory
    data_index: ImmutableIndex
    annotator: Annotator
    index_validator: IndexValidator
    sandbox_id: SandboxId = ""
    introduced_by: ImmutableIndex = -1
    introduced_via: str = "bootstrap"

    @property
    def index(self) -> ImmutableIndex:
        return self.data_index


def _is_empty_data(data: ImmutableData) -> bool:
    if data is None:
        return True
    if data == "" or data == b"":
        return True
    if isinstance(data, (list, tuple, dict, set, frozenset)) and len(data) == 0:
        return True
    return False


def admit_memory(
    data: ImmutableData,
    viewpoint: ViewpointMemory,
    data_validator: DataValidator,
    annotator: Annotator,
    index: ImmutableIndex,
    index_validator: IndexValidator,
    *,
    sandbox_id: SandboxId = "",
    introduced_by: ImmutableIndex = -1,
    introduced_via: str = "bootstrap",
) -> AdmittedMemory | None:
    """
    Admit non-empty data under a ViewpointMemory (implementation guarantee).

    Pure packer: Pool.introduce should supply index and provenance.
    """
    if not isinstance(viewpoint, ViewpointMemory):
        return None
    if _is_empty_data(data):
        return None
    if not data_validator(data):
        return None
    if not index_validator(index):
        return None
    return AdmittedMemory(
        data=data,
        viewpoint=viewpoint,
        data_index=index,
        annotator=annotator,
        index_validator=index_validator,
        sandbox_id=sandbox_id,
        introduced_by=introduced_by,
        introduced_via=introduced_via,
    )
