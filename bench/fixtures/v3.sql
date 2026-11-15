BEGIN TRANSACTION;
CREATE TABLE event_fact_tags (
                event_id INTEGER PRIMARY KEY REFERENCES events(id),
                extractor TEXT NOT NULL, facts TEXT NOT NULL
            );
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT,
    actor TEXT,
    event_kind TEXT,
    kind TEXT,
    body TEXT NOT NULL,
    config_key TEXT,
    config_value TEXT,
    stage TEXT,
    topic TEXT,
    artifact_path TEXT,
    artifact_hash TEXT,
    artifact_size INTEGER,
    redacted INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
INSERT INTO "events" VALUES(1,NULL,NULL,NULL,NULL,'synthetic baseline definition',NULL,NULL,NULL,NULL,NULL,NULL,NULL,0,0);
CREATE TABLE long_term (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    body TEXT NOT NULL,
    mid_ids TEXT NOT NULL,
    topic TEXT,
    superseded_by INTEGER,
    superseded_by_event INTEGER,
    created_at INTEGER NOT NULL
);
CREATE TABLE mid_term (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT,
    start_event_id INTEGER NOT NULL,
    end_event_id INTEGER NOT NULL,
    body TEXT NOT NULL,
    topic TEXT,
    promoted_long_id INTEGER,
    superseded_by INTEGER,
    superseded_by_event INTEGER,
    created_at INTEGER NOT NULL
);
CREATE TABLE slots (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    event_id INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE task_compactions(task_id TEXT PRIMARY KEY, through_turn INTEGER NOT NULL);
CREATE TABLE task_hints (
    task_id TEXT NOT NULL,
    event_id INTEGER NOT NULL,
    hint TEXT NOT NULL,
    PRIMARY KEY (task_id, event_id)
);
CREATE TABLE tasks (
    task_id TEXT PRIMARY KEY,
    goal TEXT NOT NULL DEFAULT '',
    progress TEXT NOT NULL DEFAULT '',
    next_step TEXT NOT NULL DEFAULT '',
    changed_event INTEGER NOT NULL UNIQUE
);
CREATE TABLE user_turns(turn INTEGER PRIMARY KEY, first_event INTEGER NOT NULL UNIQUE);
CREATE INDEX config_observations ON events(config_key,id) WHERE config_key IS NOT NULL;
CREATE INDEX event_capture_id ON events(topic,event_kind) WHERE actor='harness';
CREATE INDEX mid_bounds ON mid_term(start_event_id,end_event_id);
CREATE VIRTUAL TABLE event_index USING fts5(body, topic, config_key, stage, content='events', content_rowid='id');
INSERT INTO event_index(event_index) VALUES ('rebuild');
CREATE TRIGGER index_appended_event AFTER INSERT ON events BEGIN
    INSERT INTO event_index(rowid, body, topic, config_key, stage)
    VALUES (new.id, new.body, new.topic, new.config_key, new.stage);
END;
DELETE FROM "sqlite_sequence";
INSERT INTO "sqlite_sequence" VALUES('events',1);
COMMIT;
PRAGMA user_version=3;
