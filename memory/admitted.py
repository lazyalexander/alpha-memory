"""
AdmittedMemory — legal system payload (layer A).

Meaning
-------
Data the system has accepted as legitimate. MVP payload is ``str`` only.
Content is immutable after creation. Validation rules are out of scope here.

This module holds **data only** (no methods, no ops). Construction and
admission logic live elsewhere.
"""

from __future__ import annotations

from typing import NamedTuple


class AdmittedMemory(NamedTuple):
    """Immutable admitted payload record."""

    admitted_index: int
    data: str
    from_agent: int
