"""
I/O cost accounting and storage layout policy.

Responsibility
--------------
Track or estimate read/write amplification, batch appends, vacuum/checkpoint
policy for SQLite, and layout choices (one DB file per sandbox vs multi-tenant).
Does not change domain semantics of A/N/O — only physical cost trade-offs.
"""
