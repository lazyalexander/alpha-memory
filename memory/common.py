"""
Shared scalar aliases for the memory package.

Not a memory kind. Kind modules may import aliases from here; prefer plain
builtins on records when no shared name is needed.
"""

from __future__ import annotations

# Identity / version numbers
ImmutableIndex = int

# Wall / logical time
NumericTimestamp = int | float

# Collaboration domain mark
SandboxId = str

# Discrete feature labels (immutable set)
MemoryTags = frozenset[str]
