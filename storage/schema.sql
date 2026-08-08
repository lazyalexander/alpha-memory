-- A/N/O relational persistence (SQLite).
-- App rule: INSERT only for domain rows (no in-place update of mem content).
-- Enable: PRAGMA foreign_keys = ON;

-- Layer A: legal payload (str MVP)
CREATE TABLE IF NOT EXISTS admitted (
    admitted_index  INTEGER PRIMARY KEY NOT NULL,
    data            TEXT    NOT NULL,
    from_agent      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_admitted_from_agent
    ON admitted (from_agent);

-- Layer N: git-style node; pointer to admitted only (no embedded payload)
CREATE TABLE IF NOT EXISTS node (
    node_index          INTEGER PRIMARY KEY NOT NULL,
    sandbox_id          TEXT    NOT NULL,
    timestamp           REAL    NOT NULL,
    parent_node_index   INTEGER NULL,
    admitted_index      INTEGER NOT NULL,
    FOREIGN KEY (parent_node_index) REFERENCES node (node_index),
    FOREIGN KEY (admitted_index) REFERENCES admitted (admitted_index)
);

CREATE INDEX IF NOT EXISTS idx_node_sandbox
    ON node (sandbox_id);
CREATE INDEX IF NOT EXISTS idx_node_admitted
    ON node (admitted_index);
CREATE INDEX IF NOT EXISTS idx_node_parent
    ON node (parent_node_index);

-- Layer O: agent×viewpoint observation metadata (no reading body, no score)
-- tags stored as JSON array text, e.g. ["a","b"]
CREATE TABLE IF NOT EXISTS observation (
    observation_index         INTEGER PRIMARY KEY NOT NULL,
    agent_id                  INTEGER NOT NULL,
    viewpoint_index           INTEGER NOT NULL,
    source_node_index         INTEGER NOT NULL,
    source_observation_index  INTEGER NULL,
    tags_json                 TEXT    NOT NULL DEFAULT '[]',
    timestamp                 REAL    NOT NULL,
    FOREIGN KEY (source_node_index) REFERENCES node (node_index),
    FOREIGN KEY (source_observation_index) REFERENCES observation (observation_index)
);

CREATE INDEX IF NOT EXISTS idx_observation_agent_vp
    ON observation (agent_id, viewpoint_index);
CREATE INDEX IF NOT EXISTS idx_observation_node
    ON observation (source_node_index);
CREATE INDEX IF NOT EXISTS idx_observation_parent_obs
    ON observation (source_observation_index);
