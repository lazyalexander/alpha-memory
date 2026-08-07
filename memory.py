"""
Memory substrate: immutable data, viewpoint-as-memory, abstract lineage.

Non-negotiable axioms
---------------------
1. Original data is pass-only; never mutated after supply.
2. Viewpoint is memory. admit_memory requires a ViewpointMemory (no naked data).
3. Genesis: bootstrap_viewpoint is the only path that creates a viewpoint
   without a prior ViewpointMemory.
4. AbstractMemory is recursive via origin; source always points at the shared
   AdmittedMemory (structural sharing — do not copy data).
5. Multi-branch only (no merge). Prefer re-abstracting from original memory
   when a viewpoint changes.
6. Viewpoint identity for match/stale is (mark.token, digest), not function names.
7. Branch-root AbstractMemory carries timestamp; higher abstract nodes use
   timestamp=None (no semantic clock at abstract layers).
"""

from __future__ import annotations

from typing import Any, Callable, Iterator, NamedTuple

# --- Immutability contracts (interface + convention; not fully runtime-enforced) ---

ImmutableData = Any
"""Memory payload. Pass-only: must not be modified after supply / admission."""

ImmutableIndex = int
"""Memory index. Immutable integer value."""

ImmutableAnnotation = str
"""Annotation text. str is immutable; treat as read-only."""

MemoryTags = frozenset[str]
"""Tag set. frozenset: contents must not change."""

NumericTimestamp = int | float
"""Numeric timestamp (e.g. unix seconds). Immutable value."""

FunctionToken = bytes
"""Opaque function identity. Not a human name / __qualname__."""

DataValidator = Callable[[ImmutableData], bool]
Annotator = Callable[[ImmutableData], ImmutableAnnotation]
IndexValidator = Callable[[ImmutableIndex], bool]


class FunctionMark(NamedTuple):
    """
    Locator for a processing function — not a function name.

    - fn: live callable handle (in-process location).
    - token: opaque id for stable identity and registry lookup.
    Equality for identity policies should use token (and ViewpointMemory.digest).
    """

    fn: Callable[..., Any]
    token: FunctionToken


# --- Viewpoint-as-memory -------------------------------------------------------


class ViewpointMemory(NamedTuple):
    """
    A viewpoint (interpretation perspective) as first-class memory.

    Pass-only / immutable. Identity for allowlist and staleness:
    (mark.token, digest) — not __name__.
    """

    mark: FunctionMark
    digest: bytes
    index: ImmutableIndex
    tags: MemoryTags
    timestamp: NumericTimestamp
    meta: ImmutableAnnotation


# Viewpoint apply API: numbers, tags, timestamps the admitted payload in this view.
ViewpointFn = Callable[
    ["AdmittedMemory"],
    tuple[ImmutableIndex, MemoryTags, NumericTimestamp] | None,
]


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

    This is the only admission-free constructor for viewpoints. The system must
    seed at least one genesis viewpoint; all admit_memory calls require one.
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


def is_stale(node: AbstractMemory, current: ViewpointMemory) -> bool:
    """
    True if node's recorded viewpoint no longer matches current registration.

    Stale chains must not be patched; re-admit / re-apply from original data.
    """
    return not viewpoint_matches(node.viewpoint, current)


# --- Admission -----------------------------------------------------------------


class AdmittedMemory(NamedTuple):
    """
    Admitted payload under a required ViewpointMemory.

    data is shared by all AbstractMemory branches from this source.
    viewpoint is embedded by reference (structural share).
    """

    data: ImmutableData
    viewpoint: ViewpointMemory
    index: ImmutableIndex
    annotator: Annotator
    index_validator: IndexValidator


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
) -> AdmittedMemory | None:
    """
    Admit non-empty data under a ViewpointMemory (implementation guarantee).

    Requires a non-null ViewpointMemory — naked data cannot enter the system.
    On success returns AdmittedMemory embedding viewpoint by reference.
    Does not tag/timestamp the payload interpretation (that is apply_viewpoint).
    Failure → None.
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
        index=index,
        annotator=annotator,
        index_validator=index_validator,
    )


# --- Abstract lineage (multi-branch, recursive origin) -------------------------


class AbstractCoreInfo(NamedTuple):
    """Pass-only slice of an AbstractMemory node without lineage pointers."""

    viewpoint_token: FunctionToken
    viewpoint_digest: bytes
    index: ImmutableIndex
    tags: MemoryTags
    timestamp: NumericTimestamp | None


class AbstractMemory(NamedTuple):
    """
    Recursive abstract / viewpoint-application node (git-like commit, no merge).

    - source: always the shared AdmittedMemory (original mem); never None
    - viewpoint: perspective responsible for this node
    - origin: parent AbstractMemory, or None if branch root from admitted
    - timestamp: set on branch roots; None on higher abstract nodes
    Structural sharing: store references only; never deep-copy source.data.
    """

    source: AdmittedMemory
    viewpoint: ViewpointMemory
    index: ImmutableIndex
    tags: MemoryTags
    timestamp: NumericTimestamp | None
    origin: AbstractMemory | None


def core_info(node: AbstractMemory) -> AbstractCoreInfo:
    return AbstractCoreInfo(
        viewpoint_token=node.viewpoint.mark.token,
        viewpoint_digest=node.viewpoint.digest,
        index=node.index,
        tags=node.tags,
        timestamp=node.timestamp,
    )


def apply_viewpoint(admitted: AdmittedMemory) -> AbstractMemory | None:
    """
    Apply the embedded viewpoint function: number, tag, timestamp in that view.

    Branch root: origin=None, timestamp required (from viewpoint fn).
    Failure (fn returns None / bad shape) → None.
    """
    if not isinstance(admitted, AdmittedMemory):
        return None
    fn = admitted.viewpoint.mark.fn
    try:
        result = fn(admitted)
    except Exception:
        return None
    if result is None:
        return None
    try:
        index, tags, timestamp = result
    except (TypeError, ValueError):
        return None
    if timestamp is None:
        return None
    tag_set = tags if isinstance(tags, frozenset) else frozenset(tags)
    return AbstractMemory(
        source=admitted,
        viewpoint=admitted.viewpoint,
        index=index,
        tags=tag_set,
        timestamp=timestamp,
        origin=None,
    )


# declare(core, tags, viewpoint) -> (fc, abstractor_viewpoint) | None
# fc: concrete abstraction body over original data; may be invoked for effect.
AbstractionMethod = Callable[[ImmutableData], Any]
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
    """
    Higher abstraction over a prior AbstractMemory.

    declare may refuse (None) if the prior viewpoint is not allowed.
    On success packs a new node with origin=prior, same source, timestamp=None.
    Abstractor identity is a ViewpointMemory (homogeneous with viewpoints).
    """
    if not isinstance(prior, AbstractMemory):
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
    # Invoke fc on original data (effect / derived value); packing is lineage.
    try:
        fc(prior.source.data)
    except Exception:
        return None
    tag_set = frozenset() if tags is None else frozenset(tags)
    return AbstractMemory(
        source=prior.source,
        viewpoint=abstractor,
        index=index,
        tags=tag_set,
        timestamp=None,
        origin=prior,
    )


# --- Lineage helpers -----------------------------------------------------------


def walk_origin(node: AbstractMemory) -> Iterator[AbstractMemory]:
    """Yield node, then origin, then origin.origin, … (traceability walk)."""
    current: AbstractMemory | None = node
    while current is not None:
        yield current
        current = current.origin


def root_of(node: AbstractMemory) -> AbstractMemory:
    """Walk to the branch root (origin is None)."""
    current = node
    while current.origin is not None:
        current = current.origin
    return current


def same_source(a: AbstractMemory, b: AbstractMemory) -> bool:
    """True if both nodes share the same AdmittedMemory object."""
    return a.source is b.source


def reabstract(
    data: ImmutableData,
    viewpoint: ViewpointMemory,
    data_validator: DataValidator,
    annotator: Annotator,
    admit_index: ImmutableIndex,
    index_validator: IndexValidator,
) -> AbstractMemory | None:
    """
    Re-abstract from original data under a (possibly new) viewpoint.

    admit + apply_viewpoint. Never mutates old chains; builds a new branch.
    """
    admitted = admit_memory(
        data,
        viewpoint,
        data_validator,
        annotator,
        admit_index,
        index_validator,
    )
    if admitted is None:
        return None
    return apply_viewpoint(admitted)


# Alias: branch root from admitted
branch_root = apply_viewpoint
