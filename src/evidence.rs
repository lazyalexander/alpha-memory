//! Explicit, read-only access to a session's last observed values and source events.
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::error::{message, Result};
use crate::store::{map_event, read_session, validate_id};
use crate::{Engine, MidTerm, Slot, StoredEvent};

#[derive(Debug, Serialize)]
pub struct IndexHit {
    pub event_id: i64,
    pub task_id: Option<String>,
    pub topic: Option<String>,
    pub event_kind: Option<String>,
    pub config_key: Option<String>,
    pub preview: String,
}

#[derive(Debug, Serialize)]
pub struct SourceEvent {
    #[serde(flatten)]
    pub event: StoredEvent,
    /// Separate from the host record. An empty fact list is a completed abstention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inferred_tags: Option<crate::InferredTags>,
    /// Adjacent observations of this exact key in this session, regardless of task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_config_event: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_config_event: Option<i64>,
}

impl Engine {
    pub fn session_config(&self, session: &str, key: &str) -> Result<Option<Slot>> {
        let Some(conn) = read_session(self.workspace(), session)? else {
            return Ok(None);
        };
        Ok(conn
            .query_row(
                "SELECT key,value,event_id FROM slots WHERE key=?1",
                [key],
                |r| {
                    Ok(Slot {
                        key: r.get(0)?,
                        value: r.get(1)?,
                        event_id: r.get(2)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn evidence_index(
        &self,
        session: &str,
        query: &str,
        task: Option<&str>,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Vec<IndexHit>> {
        let query = query.trim();
        if query.is_empty() || query.len() > 512 || !(1..=20).contains(&limit) {
            return Err(message(
                "index needs 1–512 bytes of query text and a limit of 1–20",
            ));
        }
        if let Some(id) = task {
            validate_id(id, "task id")?;
        }
        let Some(conn) = read_session(self.workspace(), session)? else {
            return Ok(vec![]);
        };
        let before = before.unwrap_or(i64::MAX);
        // Treat model-supplied search terms literally, including FTS operators and quotes.
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" OR ");
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let map = |r: &rusqlite::Row<'_>| {
            Ok(IndexHit {
                event_id: r.get(0)?,
                task_id: r.get(1)?,
                topic: r.get(2)?,
                preview: r.get(3)?,
                event_kind: r.get(4)?,
                config_key: r.get(5)?,
            })
        };
        if version >= 1 {
            let mut stmt = conn.prepare(
                "SELECT e.id,e.task_id,e.topic,substr(snippet(event_index,0,'','','…',16),1,200),e.event_kind,e.config_key
                 FROM event_index JOIN events e ON e.id=event_index.rowid
                 WHERE event_index MATCH ?1 AND (?2 IS NULL OR e.task_id=?2) AND e.id<?3
                 ORDER BY (e.topic=?5 OR e.config_key=?5) IS TRUE DESC, bm25(event_index,1,5,5,2),e.id DESC LIMIT ?4")?;
            let hits: Vec<_> = stmt
                .query_map(params![terms, task, before, limit, query], map)?
                .collect::<rusqlite::Result<_>>()?;
            if !hits.is_empty() {
                return Ok(hits);
            }
        }
        // Old sessions remain searchable without migrating on read. A literal fallback
        // also finds substrings inside Chinese text that unicode61 indexed as one token.
        let mut stmt = conn.prepare(
            "SELECT id,task_id,topic,substr(body,1,200),event_kind,config_key FROM events
             WHERE (instr(lower(body),lower(?1)) OR instr(lower(topic),lower(?1)) OR instr(lower(config_key),lower(?1)) OR instr(lower(stage),lower(?1)) OR config_value=?1)
             AND (?2 IS NULL OR task_id=?2) AND id<?3 ORDER BY id DESC LIMIT ?4")?;
        let rows = stmt.query_map(params![query, task, before, limit], map)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Resolve a long-term summary's mid ID even after promotion or supersession.
    pub fn source_span(&self, session: &str, mid: i64) -> Result<MidTerm> {
        let conn = read_session(self.workspace(), session)?
            .ok_or_else(|| message("source span not found"))?;
        crate::store::load_mid(&conn, mid)
    }

    pub fn source(&self, session: &str, event: i64, around: u32) -> Result<Vec<SourceEvent>> {
        if event <= 0 || around > 3 {
            return Err(message(
                "source needs a positive event id and a radius of 0–3",
            ));
        }
        let Some(conn) = read_session(self.workspace(), session)? else {
            return Err(message("source event not found"));
        };
        let radius = i64::from(around);
        let mut stmt = conn.prepare(
            "SELECT id,task_id,actor,event_kind,kind,body,config_key,config_value,stage,topic,
            artifact_path,artifact_hash,artifact_size,redacted,created_at,
            (SELECT max(id) FROM events WHERE config_key=e.config_key AND id<e.id),
            (SELECT min(id) FROM events WHERE config_key=e.config_key AND id>e.id)
            FROM events AS e WHERE e.id BETWEEN ?1 AND ?2 ORDER BY e.id",
        )?;
        let mut events: Vec<_> = stmt
            .query_map(
                params![event.saturating_sub(radius), event.saturating_add(radius)],
                |r| {
                    Ok(SourceEvent {
                        event: map_event(r)?,
                        inferred_tags: None,
                        previous_config_event: r.get(15)?,
                        next_config_event: r.get(16)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<_>>()?;
        if !events.iter().any(|e| e.event.id == event) {
            return Err(message("source event not found"));
        }
        if conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))? >= 3 {
            for source in &mut events {
                source.inferred_tags = crate::tags::load(&conn, source.event.id)?;
            }
        }
        Ok(events)
    }
}
