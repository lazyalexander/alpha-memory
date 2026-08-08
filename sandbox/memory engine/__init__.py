"""
Memory Engine — I/O, persistence cost, cache, and retrieval acceleration.

Responsibility
--------------
Owns physical access paths for A/N/O: SQLite (or future backends), append
paths, secondary index/cache tables, activation of hot sets, and query
acceleration over immutable primary rows.

Does not decide agent policy or act semantics — that is Memory Stream.
Domain record types remain in memory/ (data only).
"""
