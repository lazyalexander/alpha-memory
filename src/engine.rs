use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::classify::{self, Kind, Signals};
use crate::error::{message, Error, Result};
use crate::redact::redact;
use crate::store::{self, LongTerm, MidTerm, NewEvent, Note, Slot, StoredEvent};

const MAX_TEXT: usize = 256 * 1024;
pub const DEFAULT_RECENT: u32 = 5;

/// Append-only session log, plus the user directory of preferences and constraints.
///
/// The engine does not call a model. The caller supplies the intent bit and any
/// prose its own model wrote.
pub struct Engine {
    workspace: PathBuf,
    user_home: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Append {
    pub session_id: String,
    pub task_id: Option<String>,
    pub actor: Option<String>,
    pub event_kind: Option<String>,
    pub body: String,
    pub config_key: Option<String>,
    pub config_value: Option<String>,
    pub stage: Option<String>,
    pub topic: Option<String>,
    pub artifact_path: Option<String>,
    pub artifact_hash: Option<String>,
    pub artifact_size: Option<i64>,
    pub failure: bool,
    pub decision: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Appended {
    pub event_id: i64,
    pub kind: Option<Kind>,
    pub redacted: bool,
}

#[derive(Debug, Clone)]
pub struct RecallQuery {
    pub session_id: String,
    pub task_id: Option<String>,
    pub open_long_term: bool,
    pub stage: Option<String>,
    pub recent: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecallPack {
    pub standing: Vec<Note>,
    pub preferences: Vec<Note>,
    pub mid_term: Vec<MidTerm>,
    pub recent_decisions: Vec<StoredEvent>,
    pub recent_failures: Vec<StoredEvent>,
    pub config: Vec<Slot>,
    pub artifacts: Vec<Artifact>,
    pub stage: Option<StoredEvent>,
    pub long_term: Vec<LongTerm>,
    pub env: Vec<StoredEvent>,
    pub long_term_opened: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Artifact {
    pub event_id: i64,
    pub path: String,
    pub hash: String,
    pub size: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ReflectMid {
    pub session_id: String,
    pub task_id: Option<String>,
    pub start_event_id: i64,
    pub end_event_id: i64,
    pub body: Option<String>,
    pub topic: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReflectLong {
    pub session_id: String,
    pub mid_ids: Vec<i64>,
    pub body: Option<String>,
    pub topic: Option<String>,
}

impl Engine {
    pub fn open(workspace: impl Into<PathBuf>, user_home: impl Into<PathBuf>) -> Result<Self> {
        let workspace = workspace.into();
        let user_home = user_home.into();
        if !workspace.is_dir() {
            return Err(Error::MissingWorkspace(workspace));
        }
        std::fs::create_dir_all(&user_home)?;
        Ok(Self {
            workspace,
            user_home,
        })
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub fn user_home(&self) -> &Path {
        &self.user_home
    }

    pub fn append(&self, mut input: Append) -> Result<Appended> {
        store::validate_id(&input.session_id, "session id")?;
        if let Some(task) = &input.task_id {
            store::validate_id(task, "task id")?;
        }
        if input.failure && input.decision {
            return Err(message("an event is either a failure or a decision"));
        }
        input.config_key = blank_to_none(input.config_key);
        input.config_value = blank_to_none(input.config_value);
        input.stage = blank_to_none(input.stage);
        input.topic = blank_to_none(input.topic);
        input.event_kind = blank_to_none(input.event_kind);
        input.artifact_path = blank_to_none(input.artifact_path);
        input.artifact_hash = blank_to_none(input.artifact_hash);
        match (&input.config_key, &input.config_value) {
            (Some(_), Some(_)) | (None, None) => {}
            _ => return Err(message("config key and value are written together")),
        }
        match (&input.artifact_path, &input.artifact_hash) {
            (Some(_), Some(_)) | (None, None) => {}
            _ => return Err(message("artifact path and hash are written together")),
        }
        check_len("body", &input.body)?;
        if let Some(value) = input.config_value.as_deref() {
            check_len("config value", value)?;
        }

        let (body, body_redacted) = redact(&input.body);
        let (config_value, value_redacted) = match input.config_value {
            Some(value) => {
                let (redacted, changed) = redact(&value);
                (Some(redacted), changed)
            }
            None => (None, false),
        };
        let kind = classify::classify(Signals {
            failure: input.failure,
            decision: input.decision,
            event_kind: input.event_kind.as_deref(),
            config_key: input.config_key.as_deref(),
            config_value: config_value.as_deref(),
            stage: input.stage.as_deref(),
        });
        if matches!(kind, Some(Kind::Decision | Kind::Failure)) && body.is_empty() {
            return Err(message("a decision or failure needs a body"));
        }
        let has_payload = !body.is_empty()
            || input.config_key.is_some()
            || input.stage.is_some()
            || input.artifact_path.is_some()
            || kind.is_some();
        if !has_payload {
            return Err(message("nothing to record"));
        }
        let topic = input.topic.clone().or_else(|| input.config_key.clone());
        let event_id = store::insert_event(
            &self.workspace,
            &input.session_id,
            NewEvent {
                task_id: input.task_id,
                actor: input.actor,
                event_kind: input.event_kind,
                kind,
                body,
                config_key: input.config_key,
                config_value,
                stage: input.stage,
                topic,
                artifact_path: input.artifact_path,
                artifact_hash: input.artifact_hash,
                artifact_size: input.artifact_size,
                redacted: body_redacted || value_redacted,
            },
        )?;
        Ok(Appended {
            event_id,
            kind,
            redacted: body_redacted || value_redacted,
        })
    }

    pub fn recall(&self, query: RecallQuery) -> Result<RecallPack> {
        store::validate_id(&query.session_id, "session id")?;
        if let Some(task) = &query.task_id {
            store::validate_id(task, "task id")?;
        }
        let snapshot = store::load_session(&self.workspace, &query.session_id)?;
        let recent_keys = recent_config_keys(&snapshot.events, query.recent);
        let config = snapshot
            .slots
            .into_iter()
            .filter(|slot| !recent_keys.iter().any(|key| key == &slot.key))
            .collect();
        let mid_term = snapshot
            .mids
            .into_iter()
            .filter(|mid| task_visible(&mid.task_id, &query.task_id))
            .collect();
        let uncovered: Vec<StoredEvent> = snapshot
            .events
            .iter()
            .filter(|event| task_visible(&event.task_id, &query.task_id))
            .filter(|event| !is_covered(event.id, &snapshot.covered_ranges))
            .filter(|event| matches!(event.kind, Some(Kind::Decision | Kind::Failure)))
            .cloned()
            .collect();
        let (recent_decisions, recent_failures) = split_tail(uncovered);
        let preferred_stage = query.stage.clone().or_else(|| {
            snapshot
                .events
                .iter()
                .rev()
                .find(|event| event.stage.is_some())
                .and_then(|event| event.stage.clone())
        });
        let mut staged: Vec<StoredEvent> = snapshot
            .events
            .iter()
            .filter(|event| {
                event.stage.as_ref() == preferred_stage.as_ref() && preferred_stage.is_some()
            })
            .cloned()
            .collect();
        let stage = staged.pop();
        staged.reverse();
        let env = if query.open_long_term {
            staged
        } else {
            Vec::new()
        };
        let artifacts = snapshot
            .events
            .iter()
            .rev()
            .filter_map(|event| {
                Some(Artifact {
                    event_id: event.id,
                    path: event.artifact_path.clone()?,
                    hash: event.artifact_hash.clone()?,
                    size: event.artifact_size,
                })
            })
            .collect();
        Ok(RecallPack {
            standing: store::list_notes(&self.user_home, "standing")?,
            preferences: store::list_notes(&self.user_home, "preference")?,
            mid_term,
            recent_decisions,
            recent_failures,
            config,
            artifacts,
            stage,
            long_term: if query.open_long_term {
                snapshot.longs
            } else {
                Vec::new()
            },
            env,
            long_term_opened: query.open_long_term,
        })
    }

    pub fn reflect_mid(&self, input: ReflectMid) -> Result<MidTerm> {
        store::validate_id(&input.session_id, "session id")?;
        if let Some(task) = &input.task_id {
            store::validate_id(task, "task id")?;
        }
        let events = store::events_between(
            &self.workspace,
            &input.session_id,
            input.start_event_id,
            input.end_event_id,
        )?;
        if events.is_empty() {
            return Err(message("reflect range does not cover any events"));
        }
        let body = match input.body {
            Some(body) if !body.is_empty() => {
                check_len("body", &body)?;
                redact(&body).0
            }
            _ => structural_mid_body(&events),
        };
        let topic = blank_to_none(input.topic);
        store::insert_mid(
            &self.workspace,
            &input.session_id,
            input.task_id,
            input.start_event_id,
            input.end_event_id,
            body,
            topic,
        )
    }

    pub fn reflect_long(&self, input: ReflectLong) -> Result<LongTerm> {
        store::validate_id(&input.session_id, "session id")?;
        let body = match input.body {
            Some(body) if !body.is_empty() => {
                check_len("body", &body)?;
                redact(&body).0
            }
            _ => structural_long_body(&input.mid_ids),
        };
        store::insert_long(
            &self.workspace,
            &input.session_id,
            &input.mid_ids,
            body,
            blank_to_none(input.topic),
        )
    }

    pub fn add_preference(&self, text: &str) -> Result<Note> {
        self.add_note("preference", text)
    }

    pub fn add_standing(&self, text: &str) -> Result<Note> {
        self.add_note("standing", text)
    }

    pub fn list_events(&self, session_id: &str) -> Result<Vec<StoredEvent>> {
        store::validate_id(session_id, "session id")?;
        Ok(store::load_session(&self.workspace, session_id)?.events)
    }

    pub fn preferences(&self) -> Result<Vec<Note>> {
        store::list_notes(&self.user_home, "preference")
    }

    pub fn standing(&self) -> Result<Vec<Note>> {
        store::list_notes(&self.user_home, "standing")
    }

    fn add_note(&self, bucket: &str, text: &str) -> Result<Note> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(message("text is empty"));
        }
        check_len("text", trimmed)?;
        let (redacted, _) = redact(trimmed);
        store::add_note(&self.user_home, bucket, &redacted)
    }
}

pub fn default_user_home() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("ALPHA_MEMORY_HOME") {
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    home.map(|path| PathBuf::from(path).join(".alpha-memory"))
        .ok_or_else(|| message("could not resolve a home directory; set ALPHA_MEMORY_HOME"))
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn check_len(label: &str, text: &str) -> Result<()> {
    if text.len() > MAX_TEXT {
        Err(message(format!("{label} exceeds {MAX_TEXT} bytes")))
    } else {
        Ok(())
    }
}

fn recent_config_keys(events: &[StoredEvent], recent: u32) -> Vec<String> {
    if recent == 0 {
        return Vec::new();
    }
    let mut keys = Vec::new();
    for event in events.iter().rev().take(recent as usize) {
        if let Some(key) = &event.config_key {
            if !keys.contains(key) {
                keys.push(key.clone());
            }
        }
    }
    keys
}

fn task_visible(owner: &Option<String>, filter: &Option<String>) -> bool {
    match filter {
        None => true,
        Some(task) => owner.is_none() || owner.as_ref() == Some(task),
    }
}

fn is_covered(event_id: i64, ranges: &[(i64, i64)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| event_id >= *start && event_id <= *end)
}

fn split_tail(events: Vec<StoredEvent>) -> (Vec<StoredEvent>, Vec<StoredEvent>) {
    let mut latest_decision: BTreeMap<String, StoredEvent> = BTreeMap::new();
    let mut unnamed = Vec::new();
    let mut failures = Vec::new();
    for event in events {
        match event.kind {
            Some(Kind::Failure) => failures.push(event),
            Some(Kind::Decision) => match event.topic.clone() {
                Some(topic) => {
                    latest_decision.insert(topic, event);
                }
                None => unnamed.push(event),
            },
            _ => {}
        }
    }
    let mut decisions = unnamed;
    decisions.extend(latest_decision.into_values());
    decisions.sort_by_key(|event| event.id);
    (decisions, failures)
}

fn structural_mid_body(events: &[StoredEvent]) -> String {
    let mut counts = BTreeMap::new();
    for event in events {
        let label = event.kind.map(Kind::as_str).unwrap_or("log");
        *counts.entry(label).or_insert(0) += 1;
    }
    let kinds = counts
        .iter()
        .map(|(label, count)| format!("{label}={count}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut keys = Vec::new();
    for event in events {
        if let Some(key) = &event.config_key {
            if !keys.contains(key) {
                keys.push(key.clone());
            }
        }
    }
    let mut artifacts = Vec::new();
    for event in events {
        if let (Some(path), Some(hash)) = (&event.artifact_path, &event.artifact_hash) {
            artifacts.push(format!("{path} {hash}"));
        }
    }
    let start = events.first().map(|event| event.id).unwrap_or(0);
    let end = events.last().map(|event| event.id).unwrap_or(0);
    format!(
        "events {start}-{end} ({})\nkinds: {kinds}\nconfig keys: {}\nartifacts: {}",
        events.len(),
        keys.join(", "),
        artifacts.join(", ")
    )
}

fn structural_long_body(mid_ids: &[i64]) -> String {
    let ids = mid_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("mids: {ids}")
}
