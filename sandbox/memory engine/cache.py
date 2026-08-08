"""
Activation / cache layer over immutable A/N/O tables.

Responsibility
--------------
Maintain *secondary* structures (hot sets, working-set pointers, materialised
query results) used to activate memories into runtime without mutating
primary A/N/O rows. Cache invalidation is versioned by new inserts / generation
counters, not by updating domain tables.

Examples (future): agent working-set table, last-act observation pointer,
sandbox head tips (if ever needed under policy B).
"""
