"""
Shared scalar types and function marks for the memory package.

Not a memory kind. Kind modules (original / abstract / reflect) import from here.
"""

from __future__ import annotations

from typing import Any, Callable, NamedTuple

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

SandboxId = str
"""Collaboration domain id (single Sandbox; no distributed uniqueness)."""

DataValidator = Callable[[ImmutableData], bool]
Annotator = Callable[[ImmutableData], ImmutableAnnotation]
IndexValidator = Callable[[ImmutableIndex], bool]


class FunctionMark(NamedTuple):
    """
    Locator for a processing function — not a function name.

    - fn: live callable handle (in-process location).
    - token: opaque id for stable identity and registry lookup.
    """

    fn: Callable[..., Any]
    token: FunctionToken
