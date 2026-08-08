"""
Secondary indexes and retrieval acceleration for immutable mem rows.

Responsibility
--------------
Build and query auxiliary indexes (tags, agent_id, viewpoint_index,
sandbox routing, FTS later) to support search/recall without scanning
primary tables. Indexes may be rebuilt from append-only primary data;
they are not the source of truth.

Primary keys and foreign keys on A/N/O stay in storage schema; this module
owns *derived* lookup structures and Engine-side multi-observation join
acceleration under the same Node.
"""
