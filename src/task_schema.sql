CREATE TABLE tasks (
    task_id TEXT PRIMARY KEY,
    goal TEXT NOT NULL DEFAULT '',
    progress TEXT NOT NULL DEFAULT '',
    next_step TEXT NOT NULL DEFAULT '',
    changed_event INTEGER NOT NULL UNIQUE
);
CREATE TABLE task_hints (
    task_id TEXT NOT NULL,
    event_id INTEGER NOT NULL,
    hint TEXT NOT NULL,
    PRIMARY KEY (task_id, event_id)
);
CREATE VIRTUAL TABLE event_index USING fts5(
    body, topic, config_key, stage, content='events', content_rowid='id'
);
INSERT INTO event_index(event_index) VALUES('rebuild');
CREATE TRIGGER index_appended_event AFTER INSERT ON events BEGIN
    INSERT INTO event_index(rowid, body, topic, config_key, stage)
    VALUES (new.id, new.body, new.topic, new.config_key, new.stage);
END;
PRAGMA user_version = 1;
