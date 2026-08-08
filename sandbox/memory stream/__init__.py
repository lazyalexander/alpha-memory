"""
Memory Stream — orchestration of memory flows (not storage I/O).

Responsibility
--------------
Coordinates how agents act on memories: sequencing of admitted → node →
observation, branching policy, multi-observation composition under one act
viewpoint, and hand-off to the Memory Engine for persistence/cache.

Does not own SQLite, file bytes, or physical indexes. Those belong to
Memory Engine (and/or storage/).
"""
