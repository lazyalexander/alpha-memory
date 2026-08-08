"""
Composition helpers for multi-observation agent working sets.

Responsibility
--------------
When an agent holds many ObservationMemory instances (different viewpoints
or re-observation chains) under the same Node / lineage, compose them for
context or for Engine-side acceleration requests. Multi-viewpoint is always
modeled as multiple Observations, not as one Observation with many viewpoints.

Does not merge payloads or rewrite Nodes. Does not persist.
"""
