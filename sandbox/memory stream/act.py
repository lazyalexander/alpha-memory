"""
Act orchestration — agent cognition steps over Node handles.

Responsibility
--------------
Defines the act pipeline: given agent + active Viewpoint + source Node
(and optionally a parent Observation for re-observation), produce a new
ObservationMemory (append-only). Encodes single-parent observation edges
and 1:N children as a knowledge/index graph over nodes.

Does not score importance into Observation fields; Viewpoint may expose
scoring methods for callers, but scores are not persisted on Observation.
Does not talk to SQLite directly — emits records / commands for Engine.
"""
