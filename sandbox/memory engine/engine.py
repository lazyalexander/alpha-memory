"""
Memory Engine core I/O façade.

Responsibility
--------------
- Open/close stores (e.g. SQLite path per sandbox shard).
- Append-only insert of Admitted / Node / Observation.
- Point reads by primary key.
- Delegate schema and low-level SQL to storage/ (or local adapters).

No updates or deletes of A/N/O domain rows. Branching = new Node rows.
"""
