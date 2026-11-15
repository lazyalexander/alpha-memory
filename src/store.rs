use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};

use crate::classify::Kind;
use crate::error::{message, Result};

const SESSION_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS events (
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
CREATE INDEX IF NOT EXISTS config_observations ON events(config_key,id) WHERE config_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS event_capture_id ON events(topic,event_kind) WHERE actor='harness';
CREATE TABLE IF NOT EXISTS slots (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    event_id INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS mid_term (
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
CREATE INDEX IF NOT EXISTS mid_bounds ON mid_term(start_event_id,end_event_id);
CREATE TABLE IF NOT EXISTS long_term (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    body TEXT NOT NULL,
    mid_ids TEXT NOT NULL,
    topic TEXT,
    superseded_by INTEGER,
    superseded_by_event INTEGER,
    created_at INTEGER NOT NULL
);
";

const USER_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bucket TEXT NOT NULL CHECK (bucket IN ('preference', 'standing')),
    text TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (bucket, text)
);
";

#[derive(Debug, Clone)]
pub struct NewEvent {
    pub task_id: Option<String>,
    pub actor: Option<String>,
    pub event_kind: Option<String>,
    pub kind: Option<Kind>,
    pub body: String,
    pub config_key: Option<String>,
    pub config_value: Option<String>,
    pub stage: Option<String>,
    pub topic: Option<String>,
    pub artifact_path: Option<String>,
    pub artifact_hash: Option<String>,
    pub artifact_size: Option<i64>,
    pub redacted: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredEvent {
    pub id: i64,
    pub task_id: Option<String>,
    pub actor: Option<String>,
    pub event_kind: Option<String>,
    pub kind: Option<Kind>,
    pub body: String,
    pub config_key: Option<String>,
    pub config_value: Option<String>,
    pub stage: Option<String>,
    pub topic: Option<String>,
    pub artifact_path: Option<String>,
    pub artifact_hash: Option<String>,
    pub artifact_size: Option<i64>,
    pub redacted: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Slot {
    pub key: String,
    pub value: String,
    pub event_id: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MidTerm {
    pub id: i64,
    pub task_id: Option<String>,
    pub start_event_id: i64,
    pub end_event_id: i64,
    pub body: String,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LongTerm {
    pub id: i64,
    pub body: String,
    pub mid_ids: Vec<i64>,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Note {
    pub id: i64,
    pub text: String,
}

pub struct SessionSnapshot {
    pub events: Vec<StoredEvent>,
    pub slots: Vec<Slot>,
    pub mids: Vec<MidTerm>,
    pub longs: Vec<LongTerm>,
    pub covered_ranges: Vec<(i64, i64)>,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub fn session_db_path(workspace: &Path, session_id: &str) -> Result<PathBuf> {
    validate_id(session_id, "session id")?;
    Ok(workspace
        .join(".alpha-memory")
        .join("sessions")
        .join(session_id)
        .join("store.sqlite"))
}

pub fn user_db_path(user_home: &Path) -> PathBuf {
    user_home.join("user.sqlite")
}

pub fn validate_id(value: &str, label: &str) -> Result<()> {
    let ok = !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if ok {
        Ok(())
    } else {
        Err(message(format!(
            "{label} must be 1-128 characters of ASCII letters, digits, '.', '_' or '-'"
        )))
    }
}

pub fn insert_event(workspace: &Path, session_id: &str, event: NewEvent) -> Result<i64> {
    let path = session_db_path(workspace, session_id)?;
    let config_key = event.config_key.clone();
    let config_value = event.config_value.clone();
    let topic = event.topic.clone();
    let kind = event.kind;
    let mut conn = open_session(&path)?;
    let tx = conn.transaction()?;
    let created_at = now_ms();
    tx.execute(
        "INSERT INTO events (
            task_id, actor, event_kind, kind, body, config_key, config_value, stage, topic,
            artifact_path, artifact_hash, artifact_size, redacted, created_at
        ) VALUES (COALESCE(?1, (SELECT task_id FROM tasks ORDER BY changed_event DESC LIMIT 1)),
            ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            event.task_id,
            event.actor,
            event.event_kind,
            event.kind.map(Kind::as_str),
            event.body,
            event.config_key,
            event.config_value,
            event.stage,
            event.topic,
            event.artifact_path,
            event.artifact_hash,
            event.artifact_size,
            if event.redacted { 1 } else { 0 },
            created_at,
        ],
    )?;
    let event_id = tx.last_insert_rowid();
    if event.event_kind.as_deref() == Some("user_message") {
        record_user_turn(&tx, event_id)?;
    }
    if let (Some(key), Some(value)) = (config_key, config_value) {
        let previous = match tx.query_row("SELECT value FROM slots WHERE key = ?1", [&key], |row| {
            row.get::<_, String>(0)
        }) {
            Ok(value) => Some(value),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(err) => return Err(err.into()),
        };
        tx.execute(
            "INSERT INTO slots (key, value, event_id, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, event_id = excluded.event_id, updated_at = excluded.updated_at",
            params![&key, &value, event_id, created_at],
        )?;
        if previous.as_ref().is_some_and(|old| old != &value) {
            supersede_topic(&tx, &key, event_id)?;
        }
    }
    if kind == Some(Kind::Decision) {
        if let Some(topic) = topic {
            supersede_topic(&tx, &topic, event_id)?;
        }
    }
    tx.commit()?;
    Ok(event_id)
}

fn supersede_topic(tx: &Connection, topic: &str, event_id: i64) -> Result<()> {
    tx.execute(
        "UPDATE mid_term SET superseded_by_event = ?1
         WHERE topic = ?2 AND promoted_long_id IS NULL AND superseded_by IS NULL AND superseded_by_event IS NULL",
        params![event_id, topic],
    )?;
    tx.execute(
        "UPDATE long_term SET superseded_by_event = ?1
         WHERE topic = ?2 AND superseded_by IS NULL AND superseded_by_event IS NULL",
        params![event_id, topic],
    )?;
    Ok(())
}

pub fn load_session(workspace: &Path, session_id: &str) -> Result<SessionSnapshot> {
    let path = session_db_path(workspace, session_id)?;
    if !path.exists() {
        return Ok(SessionSnapshot {
            events: Vec::new(),
            slots: Vec::new(),
            mids: Vec::new(),
            longs: Vec::new(),
            covered_ranges: Vec::new(),
        });
    }
    let mut conn = open_session(&path)?;
    let tx = conn.transaction()?;
    let events = query_events(&tx)?;
    let slots = query_slots(&tx)?;
    let mids = query_mids(&tx)?;
    let longs = query_longs(&tx)?;
    let covered_ranges = query_ranges(&tx)?;
    tx.commit()?;
    Ok(SessionSnapshot {
        events,
        slots,
        mids,
        longs,
        covered_ranges,
    })
}

pub fn insert_mid(
    workspace: &Path,
    session_id: &str,
    task_id: Option<String>,
    start_event_id: i64,
    end_event_id: i64,
    body: String,
    topic: Option<String>,
) -> Result<MidTerm> {
    let path = session_db_path(workspace, session_id)?;
    let mut conn = open_session(&path)?;
    let tx = conn.transaction()?;
    ensure_bounds(&tx, start_event_id, end_event_id)?;
    tx.execute(
        "INSERT INTO mid_term (task_id, start_event_id, end_event_id, body, topic, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![task_id, start_event_id, end_event_id, body, topic, now_ms()],
    )?;
    let id = tx.last_insert_rowid();
    let mid = load_mid(&tx, id)?;
    tx.commit()?;
    Ok(mid)
}

pub fn insert_long(
    workspace: &Path,
    session_id: &str,
    mid_ids: &[i64],
    body: String,
    topic: Option<String>,
) -> Result<LongTerm> {
    if mid_ids.is_empty() {
        return Err(message(
            "long-term reflection needs at least one mid-term id",
        ));
    }
    let path = session_db_path(workspace, session_id)?;
    let mut conn = open_session(&path)?;
    let tx = conn.transaction()?;
    for mid_id in mid_ids {
        let (promoted, superseded, superseded_event): (Option<i64>, Option<i64>, Option<i64>) = tx
            .query_row(
                "SELECT promoted_long_id, superseded_by, superseded_by_event FROM mid_term WHERE id = ?1",
                [mid_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| message(format!("mid-term entry {mid_id} does not exist")))?;
        if promoted.is_some() {
            return Err(message(format!(
                "mid-term entry {mid_id} is already promoted"
            )));
        }
        if superseded.is_some() || superseded_event.is_some() {
            return Err(message(format!("mid-term entry {mid_id} is superseded")));
        }
    }
    let mid_json = serde_json::to_string(mid_ids)?;
    tx.execute(
        "INSERT INTO long_term (body, mid_ids, topic, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![body, mid_json, topic, now_ms()],
    )?;
    let long_id = tx.last_insert_rowid();
    for mid_id in mid_ids {
        tx.execute(
            "UPDATE mid_term SET promoted_long_id = ?1 WHERE id = ?2",
            params![long_id, mid_id],
        )?;
    }
    if let Some(topic) = topic.as_deref() {
        tx.execute(
            "UPDATE long_term SET superseded_by = ?1
             WHERE topic = ?2 AND id != ?1 AND superseded_by IS NULL AND superseded_by_event IS NULL",
            params![long_id, topic],
        )?;
    }
    let long = load_long(&tx, long_id)?;
    tx.commit()?;
    Ok(long)
}

/// Capture an entire host part atomically. A retried hook with the same part ID
/// returns its existing event IDs, including after the user has switched tasks.
pub fn capture(
    workspace: &Path,
    session: &str,
    kind: &str,
    interaction: Option<&str>,
    text: &str,
    redacted: bool,
    chunk_bytes: usize,
) -> Result<Vec<i64>> {
    let mut conn = open_session(&session_db_path(workspace, session)?)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut ids = Vec::new();
    if let Some(interaction) = interaction {
        let mut stmt = tx.prepare("SELECT id,body FROM events WHERE actor='harness' AND event_kind=?1 AND topic=?2 ORDER BY id")?;
        let mut rows = stmt.query(params![kind, interaction])?;
        let mut remaining = text;
        while let Some(row) = rows.next()? {
            let existing: String = row.get(1)?;
            remaining = remaining
                .strip_prefix(&existing)
                .ok_or_else(|| message("capture ID already used for different text"))?;
            ids.push(row.get(0)?);
        }
        if !ids.is_empty() {
            if !remaining.is_empty() {
                return Err(message("capture ID already used for different text"));
            }
            return Ok(ids);
        }
    }
    if text.is_empty() && kind != "user_message" {
        return Ok(ids);
    }
    let mut remaining = text;
    loop {
        let mut end = remaining.len().min(chunk_bytes);
        while !remaining.is_char_boundary(end) {
            end -= 1;
        }
        let (part, tail) = remaining.split_at(end);
        tx.execute(
           "INSERT INTO events(task_id, actor, event_kind, topic, body, redacted, created_at)
            VALUES ((SELECT task_id FROM tasks ORDER BY changed_event DESC LIMIT 1),'harness',?1,?2,?3,?4,?5)",
           params![kind,interaction,part,redacted,now_ms()],
        )?;
        ids.push(tx.last_insert_rowid());
        if tail.is_empty() {
            break;
        }
        remaining = tail;
    }
    if kind == "user_message" {
        record_user_turn(&tx, ids[0])?;
    }
    tx.commit()?;
    Ok(ids)
}

fn record_user_turn(conn: &Connection, first_event: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO user_turns(first_event) VALUES (?1)",
        [first_event],
    )?;
    Ok(())
}

pub(crate) fn current_turn(conn: &Connection) -> Result<i64> {
    Ok(
        conn.query_row("SELECT COALESCE(MAX(turn),0) FROM user_turns", [], |r| {
            r.get(0)
        })?,
    )
}

pub fn reflect_pending(workspace: &Path, session: &str) -> Result<Vec<MidTerm>> {
    let path = session_db_path(workspace, session)?;
    if !path.try_exists()? {
        return Ok(Vec::new());
    }
    let mut conn = open_session(&path)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let spans = {
        // Consecutive IDs in the same task share id - row_number. Previously
        // promoted or superseded ranges still count as covered.
        let mut stmt=tx.prepare("SELECT task_id,min(id),max(id) FROM (
            SELECT e.id,e.task_id,e.id-row_number() OVER (PARTITION BY e.task_id ORDER BY e.id) AS span
            FROM events e WHERE NOT EXISTS (
                SELECT 1 FROM mid_term m WHERE e.id BETWEEN m.start_event_id AND m.end_event_id))
            GROUP BY task_id,span ORDER BY min(id)")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut written = Vec::new();
    for (task, start, end) in spans {
        // Structural reflection needs identifiers and kinds, never full transcript bodies.
        let mut stmt=tx.prepare("SELECT id,task_id,actor,event_kind,kind,'',config_key,NULL,stage,topic,
            artifact_path,artifact_hash,artifact_size,redacted,created_at FROM events WHERE id BETWEEN ?1 AND ?2 ORDER BY id")?;
        let events = stmt
            .query_map(params![start, end], map_event)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let body = crate::engine::structural_mid_body(&events);
        tx.execute("INSERT INTO mid_term(task_id,start_event_id,end_event_id,body,created_at) VALUES (?1,?2,?3,?4,?5)",
            params![task,start,end,body,now_ms()])?;
        written.push(load_mid(&tx, tx.last_insert_rowid())?);
    }
    tx.commit()?;
    Ok(written)
}

pub fn events_between(
    workspace: &Path,
    session_id: &str,
    start: i64,
    end: i64,
) -> Result<Vec<StoredEvent>> {
    let path = session_db_path(workspace, session_id)?;
    if !path.exists() {
        return Err(message("session has no events"));
    }
    let conn = open_session(&path)?;
    let mut statement = conn.prepare(
        "SELECT id, task_id, actor, event_kind, kind, body, config_key, config_value, stage, topic,
                artifact_path, artifact_hash, artifact_size, redacted, created_at
         FROM events WHERE id >= ?1 AND id <= ?2 ORDER BY id ASC",
    )?;
    let rows = statement.query_map(params![start, end], map_event)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn ensure_bounds(conn: &Connection, start: i64, end: i64) -> Result<()> {
    if start > end {
        return Err(message("reflect range start is after end"));
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM events WHERE id >= ?1 AND id <= ?2",
        params![start, end],
        |row| row.get(0),
    )?;
    if count == 0 {
        return Err(message("reflect range does not cover any events"));
    }
    Ok(())
}

fn query_events(conn: &Connection) -> Result<Vec<StoredEvent>> {
    let mut statement = conn.prepare(
        "SELECT id, task_id, actor, event_kind, kind, body, config_key, config_value, stage, topic,
                artifact_path, artifact_hash, artifact_size, redacted, created_at
         FROM events ORDER BY id ASC",
    )?;
    let rows = statement.query_map([], map_event)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn query_slots(conn: &Connection) -> Result<Vec<Slot>> {
    let mut statement = conn.prepare("SELECT key, value, event_id FROM slots ORDER BY key ASC")?;
    let rows = statement.query_map([], |row| {
        Ok(Slot {
            key: row.get(0)?,
            value: row.get(1)?,
            event_id: row.get(2)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn query_mids(conn: &Connection) -> Result<Vec<MidTerm>> {
    let mut statement = conn.prepare(
        "SELECT id, task_id, start_event_id, end_event_id, body, topic
         FROM mid_term
         WHERE promoted_long_id IS NULL AND superseded_by IS NULL AND superseded_by_event IS NULL
         ORDER BY id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(MidTerm {
            id: row.get(0)?,
            task_id: row.get(1)?,
            start_event_id: row.get(2)?,
            end_event_id: row.get(3)?,
            body: row.get(4)?,
            topic: row.get(5)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn query_longs(conn: &Connection) -> Result<Vec<LongTerm>> {
    let mut statement = conn.prepare(
        "SELECT id, body, mid_ids, topic FROM long_term
         WHERE superseded_by IS NULL AND superseded_by_event IS NULL
         ORDER BY id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        let mid_ids: String = row.get(2)?;
        Ok(LongTerm {
            id: row.get(0)?,
            body: row.get(1)?,
            mid_ids: serde_json::from_str(&mid_ids).unwrap_or_default(),
            topic: row.get(3)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn query_ranges(conn: &Connection) -> Result<Vec<(i64, i64)>> {
    let mut statement =
        conn.prepare("SELECT start_event_id, end_event_id FROM mid_term ORDER BY id ASC")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub(crate) fn load_mid(conn: &Connection, id: i64) -> Result<MidTerm> {
    conn.query_row(
        "SELECT id, task_id, start_event_id, end_event_id, body, topic FROM mid_term WHERE id = ?1",
        [id],
        |row| {
            Ok(MidTerm {
                id: row.get(0)?,
                task_id: row.get(1)?,
                start_event_id: row.get(2)?,
                end_event_id: row.get(3)?,
                body: row.get(4)?,
                topic: row.get(5)?,
            })
        },
    )
    .map_err(Into::into)
}

fn load_long(conn: &Connection, id: i64) -> Result<LongTerm> {
    conn.query_row(
        "SELECT id, body, mid_ids, topic FROM long_term WHERE id = ?1",
        [id],
        |row| {
            let mid_ids: String = row.get(2)?;
            Ok(LongTerm {
                id: row.get(0)?,
                body: row.get(1)?,
                mid_ids: serde_json::from_str(&mid_ids).unwrap_or_default(),
                topic: row.get(3)?,
            })
        },
    )
    .map_err(Into::into)
}

pub(crate) fn map_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredEvent> {
    let kind: Option<String> = row.get(4)?;
    Ok(StoredEvent {
        id: row.get(0)?,
        task_id: row.get(1)?,
        actor: row.get(2)?,
        event_kind: row.get(3)?,
        kind: kind.as_deref().and_then(Kind::parse),
        body: row.get(5)?,
        config_key: row.get(6)?,
        config_value: row.get(7)?,
        stage: row.get(8)?,
        topic: row.get(9)?,
        artifact_path: row.get(10)?,
        artifact_hash: row.get(11)?,
        artifact_size: row.get(12)?,
        redacted: row.get::<_, i64>(13)? != 0,
        created_at: row.get(14)?,
    })
}

pub fn add_note(user_home: &Path, bucket: &str, text: &str) -> Result<Note> {
    let path = user_db_path(user_home);
    let conn = open_db(&path, USER_SCHEMA)?;
    conn.execute(
        "INSERT INTO notes (bucket, text, created_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(bucket, text) DO NOTHING",
        params![bucket, text, now_ms()],
    )?;
    conn.query_row(
        "SELECT id, text FROM notes WHERE bucket = ?1 AND text = ?2",
        params![bucket, text],
        |row| {
            Ok(Note {
                id: row.get(0)?,
                text: row.get(1)?,
            })
        },
    )
    .map_err(Into::into)
}

pub fn list_notes(user_home: &Path, bucket: &str) -> Result<Vec<Note>> {
    let path = user_db_path(user_home);
    let Some(conn) = read_existing(&path)? else {
        return Ok(Vec::new());
    };
    let mut statement =
        conn.prepare("SELECT id, text FROM notes WHERE bucket = ?1 ORDER BY id ASC")?;
    let rows = statement.query_map([bucket], |row| {
        Ok(Note {
            id: row.get(0)?,
            text: row.get(1)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

// Migrate on the first write. Read-only callers can still inspect older sessions.
pub(crate) fn open_session(path: &Path) -> Result<Connection> {
    let mut conn = open_db(path, SESSION_SCHEMA)?;
    if conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))? < 3 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 0 {
            tx.execute_batch(include_str!("task_schema.sql"))?;
        }
        if version < 2 {
            // Older checkpoints do not establish which messages were compressed.
            // Start a local clock on upgrade; future differences remain exact.
            tx.execute_batch("CREATE TABLE user_turns(turn INTEGER PRIMARY KEY, first_event INTEGER NOT NULL UNIQUE);
                CREATE TABLE task_compactions(task_id TEXT PRIMARY KEY, through_turn INTEGER NOT NULL);
                PRAGMA user_version=2;")?;
        }
        if version < 3 {
            tx.execute_batch(
                "CREATE TABLE event_fact_tags (
                event_id INTEGER PRIMARY KEY REFERENCES events(id),
                extractor TEXT NOT NULL, facts TEXT NOT NULL
            ); PRAGMA user_version=3;",
            )?;
        }
        tx.commit()?;
    }
    Ok(conn)
}

pub(crate) fn read_session(workspace: &Path, session: &str) -> Result<Option<Connection>> {
    read_existing(&session_db_path(workspace, session)?)
}

fn read_existing(path: &Path) -> Result<Option<Connection>> {
    if !path.try_exists()? {
        return Ok(None);
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(Some(conn))
}

fn open_db(path: &Path, schema: &str) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(schema)?;
    Ok(conn)
}
