"""
Memory Stream core orchestration surface.

Responsibility
--------------
Top-level API for memory orchestration inside a sandbox session:
- accept only Admitted as the single payload ingress (via upstream admit);
- drive act flows that produce ObservationMemory under one active Viewpoint;
- allow an agent to hold many Observations (different viewpoints / parents)
  while enforcing one Viewpoint per act interpretation;
- never mutate A/N/O rows; only request appends through Memory Engine.

Implementation deferred. Keep free functions / pure coordination here —
not methods on memory NamedTuples.
"""
