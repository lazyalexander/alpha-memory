CREATE TABLE claims (
    version INTEGER PRIMARY KEY REFERENCES events(id),
    claim_key TEXT NOT NULL,
    previous INTEGER UNIQUE REFERENCES claims(version),
    payload TEXT NOT NULL
);
CREATE UNIQUE INDEX claim_root ON claims(claim_key) WHERE previous IS NULL;
CREATE TABLE checks (
    event INTEGER PRIMARY KEY REFERENCES events(id),
    target INTEGER NOT NULL REFERENCES claims(version),
    payload TEXT NOT NULL
);
CREATE INDEX checks_target ON checks(target);
CREATE TABLE arguments (
    event INTEGER PRIMARY KEY REFERENCES events(id),
    target INTEGER NOT NULL REFERENCES claims(version),
    payload TEXT NOT NULL
);
CREATE INDEX arguments_target ON arguments(target);
CREATE TABLE claim_requests (
    request TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    payload TEXT NOT NULL,
    result TEXT NOT NULL
);
PRAGMA user_version=4;
