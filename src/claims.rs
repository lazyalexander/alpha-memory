//! Immutable, session-local claims. Checkers execute outside the memory engine.
use crate::error::message;
use crate::store::{self, now_ms, validate_id};
use crate::{Engine, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimInput {
    pub key: String,
    pub definition: String,
    /// Opaque domain-owned scope. Equality is exact, never inferred containment.
    pub scope: String,
    pub previous: Option<i64>,
    pub sources: Vec<i64>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Complete,
    Partial,
    Unknown,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Support,
    Refute,
    Timeout,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckInput {
    pub target: i64,
    pub checker: String,
    pub checker_version: String,
    pub scope: Option<String>,
    pub coverage: Coverage,
    pub method: String,
    pub outcome: CheckOutcome,
    pub sources: Vec<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArgumentInput {
    pub target: i64,
    /// Each premise is necessary; distinct arguments are independent alternatives.
    pub premises: Vec<i64>,
    pub sources: Vec<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Record<T> {
    pub event: i64,
    pub data: T,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimView {
    pub claim: Record<ClaimInput>,
    pub checks: Vec<Record<CheckInput>>,
    pub arguments: Vec<Record<ArgumentInput>>,
    pub sources: Vec<SourceRecord>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRecord {
    pub event: i64,
    pub body: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Unsupported,
    Sampled,
    Supported,
    Verified,
    Refuted,
}
impl ClaimStatus {
    fn supports(self) -> bool {
        matches!(self, Self::Supported | Self::Verified)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArgumentState {
    pub event: i64,
    pub valid: bool,
    /// Exact premise versions and their computed states, including failed premises.
    pub premises: BTreeMap<i64, ClaimStatus>,
    pub sources: Vec<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Explanation {
    pub version: i64,
    pub status: ClaimStatus,
    pub supporting_checks: Vec<i64>,
    pub partial_checks: Vec<i64>,
    pub counterexamples: Vec<i64>,
    pub inconclusive_checks: Vec<i64>,
    pub arguments: Vec<ArgumentState>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimGraph {
    pub roots: Vec<i64>,
    pub claims: BTreeMap<i64, ClaimView>,
    pub states: BTreeMap<i64, Explanation>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Handoff {
    pub format: u32,
    pub workspace: String,
    pub source_session: String,
    pub source_position: i64,
    pub graph: ClaimGraph,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffRef {
    pub source_session: String,
    pub event: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedHandoff {
    pub reference: HandoffRef,
    pub manifest: Handoff,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateChange {
    pub version: i64,
    pub before: Explanation,
    pub after: Explanation,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoredHandoff {
    pub reference: HandoffRef,
    pub receipt_event: i64,
    pub exported_position: i64,
    pub checked_position: i64,
    pub exported: ClaimGraph,
    pub current: ClaimGraph,
    pub changes: Vec<StateChange>,
}

fn text(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 256 * 1024 {
        return Err(message(format!("invalid {label}")));
    }
    // Reject rather than change identity-bearing JSON or scope strings.
    if crate::redact::redact(value).1 {
        return Err(message(format!(
            "{label} contains secret-shaped text; use redacted source records"
        )));
    }
    Ok(())
}
fn source(conn: &Connection, id: i64) -> Result<SourceRecord> {
    conn.query_row("SELECT id,body FROM events WHERE id=?1", [id], |r| {
        Ok(SourceRecord {
            event: r.get(0)?,
            body: r.get(1)?,
        })
    })
    .optional()?
    .ok_or_else(|| message(format!("missing source event {id}")))
}
fn sources(conn: &Connection, ids: &[i64]) -> Result<()> {
    if ids.is_empty() {
        return Err(message("at least one source event is required"));
    }
    for id in ids {
        source(conn, *id)?;
    }
    Ok(())
}
fn event(conn: &Connection, kind: &str, body: &str) -> Result<i64> {
    conn.execute("INSERT INTO events(actor,event_kind,body,redacted,created_at) VALUES ('memory',?1,?2,0,?3)", params![kind,body,now_ms()])?;
    Ok(conn.last_insert_rowid())
}
fn cached<T: for<'de> Deserialize<'de>>(
    conn: &Connection,
    request: &str,
    op: &str,
    payload: &str,
) -> Result<Option<T>> {
    validate_id(request, "request id")?;
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT operation,payload,result FROM claim_requests WHERE request=?1",
            [request],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some((a, b, c)) if a == op && b == payload => Ok(Some(serde_json::from_str(&c)?)),
        Some(_) => Err(message(
            "request id already used with different operation or payload",
        )),
    }
}
fn remember(
    conn: &Connection,
    request: &str,
    op: &str,
    payload: &str,
    result: &impl Serialize,
) -> Result<()> {
    conn.execute(
        "INSERT INTO claim_requests VALUES (?1,?2,?3,?4)",
        params![request, op, payload, serde_json::to_string(result)?],
    )?;
    Ok(())
}
fn claim(conn: &Connection, version: i64) -> Result<Record<ClaimInput>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT payload FROM claims WHERE version=?1",
            [version],
            |r| r.get(0),
        )
        .optional()?;
    Ok(Record {
        event: version,
        data: serde_json::from_str(
            &raw.ok_or_else(|| message(format!("missing claim version {version}")))?,
        )?,
    })
}
fn records<T: for<'de> Deserialize<'de>>(
    conn: &Connection,
    sql: &str,
    target: i64,
) -> Result<Vec<Record<T>>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([target], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    rows.map(|row| {
        let (event, data) = row?;
        Ok(Record {
            event,
            data: serde_json::from_str(&data)?,
        })
    })
    .collect()
}
fn view(conn: &Connection, version: i64) -> Result<ClaimView> {
    let claim = claim(conn, version)?;
    let checks: Vec<Record<CheckInput>> = records(
        conn,
        "SELECT event,payload FROM checks WHERE target=?1 ORDER BY event",
        version,
    )?;
    let arguments: Vec<Record<ArgumentInput>> = records(
        conn,
        "SELECT event,payload FROM arguments WHERE target=?1 ORDER BY event",
        version,
    )?;
    let mut ids: BTreeSet<i64> = claim.data.sources.iter().copied().collect();
    ids.insert(version);
    for c in &checks {
        ids.insert(c.event);
        ids.extend(&c.data.sources);
    }
    for a in &arguments {
        ids.insert(a.event);
        ids.extend(&a.data.sources);
    }
    Ok(ClaimView {
        claim,
        checks,
        arguments,
        sources: ids
            .into_iter()
            .map(|id| source(conn, id))
            .collect::<Result<_>>()?,
    })
}
fn collect(
    conn: &Connection,
    id: i64,
    claims: &mut BTreeMap<i64, ClaimView>,
    visiting: &mut BTreeSet<i64>,
) -> Result<()> {
    if visiting.contains(&id) {
        return Err(message("cyclic claim dependency"));
    }
    if claims.contains_key(&id) {
        return Ok(());
    }
    visiting.insert(id);
    let v = view(conn, id)?;
    if let Some(previous) = v.claim.data.previous {
        collect(conn, previous, claims, visiting)?;
    }
    for a in &v.arguments {
        for p in &a.data.premises {
            collect(conn, *p, claims, visiting)?;
        }
    }
    visiting.remove(&id);
    claims.insert(id, v);
    Ok(())
}
fn evaluate(
    id: i64,
    claims: &BTreeMap<i64, ClaimView>,
    states: &mut BTreeMap<i64, Explanation>,
) -> Explanation {
    if let Some(s) = states.get(&id) {
        return s.clone();
    }
    let v = &claims[&id];
    let mut e = Explanation {
        version: id,
        status: ClaimStatus::Unsupported,
        supporting_checks: vec![],
        partial_checks: vec![],
        counterexamples: vec![],
        inconclusive_checks: vec![],
        arguments: vec![],
    };
    for c in &v.checks {
        match c.data.outcome {
            CheckOutcome::Refute => e.counterexamples.push(c.event),
            CheckOutcome::Support
                if c.data.coverage == Coverage::Complete
                    && c.data.scope.as_deref() == Some(&v.claim.data.scope) =>
            {
                e.supporting_checks.push(c.event)
            }
            CheckOutcome::Support if c.data.coverage == Coverage::Partial => {
                e.partial_checks.push(c.event)
            }
            _ => e.inconclusive_checks.push(c.event),
        }
    }
    for a in &v.arguments {
        let premises: BTreeMap<_, _> = a
            .data
            .premises
            .iter()
            .map(|p| (*p, evaluate(*p, claims, states).status))
            .collect();
        e.arguments.push(ArgumentState {
            event: a.event,
            valid: premises.values().all(|s| s.supports()),
            premises,
            sources: a.data.sources.clone(),
        });
    }
    e.status = if !e.counterexamples.is_empty() {
        ClaimStatus::Refuted
    } else if !e.supporting_checks.is_empty() {
        ClaimStatus::Verified
    } else if e.arguments.iter().any(|a| a.valid) {
        ClaimStatus::Supported
    } else if !e.partial_checks.is_empty() {
        ClaimStatus::Sampled
    } else {
        ClaimStatus::Unsupported
    };
    states.insert(id, e.clone());
    e
}
fn graph(conn: &Connection, roots: &[i64]) -> Result<ClaimGraph> {
    if roots.is_empty() {
        return Err(message("at least one claim version is required"));
    }
    let mut claims = BTreeMap::new();
    for id in roots {
        collect(conn, *id, &mut claims, &mut BTreeSet::new())?;
    }
    let mut states = BTreeMap::new();
    for id in claims.keys() {
        evaluate(*id, &claims, &mut states);
    }
    Ok(ClaimGraph {
        roots: roots.to_vec(),
        claims,
        states,
    })
}
fn position(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COALESCE(MAX(id),0) FROM events", [], |r| r.get(0))?)
}

impl Engine {
    pub fn claim_add(
        &self,
        session: &str,
        request: &str,
        input: ClaimInput,
    ) -> Result<Record<ClaimInput>> {
        validate_id(&input.key, "claim key")?;
        text(&input.definition, "definition")?;
        text(&input.scope, "scope")?;
        let payload = serde_json::to_string(&input)?;
        let mut conn = store::open_session(&store::session_db_path(self.workspace(), session)?)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(result) = cached(&tx, request, "claim", &payload)? {
            return Ok(result);
        }
        sources(&tx, &input.sources)?;
        if let Some(prev) = input.previous {
            if claim(&tx, prev)?.data.key != input.key {
                return Err(message("previous version belongs to a different claim"));
            }
        }
        let id = event(&tx, "claim", &payload)?;
        tx.execute(
            "INSERT INTO claims VALUES (?1,?2,?3,?4)",
            params![id, input.key, input.previous, payload],
        )?;
        let result = Record {
            event: id,
            data: input,
        };
        remember(&tx, request, "claim", &payload, &result)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn check_record(
        &self,
        session: &str,
        request: &str,
        input: CheckInput,
    ) -> Result<Record<CheckInput>> {
        text(&input.checker, "checker")?;
        text(&input.checker_version, "checker version")?;
        text(&input.method, "method")?;
        if let Some(s) = &input.scope {
            text(s, "check scope")?;
        }
        if input.coverage != Coverage::Unknown && input.scope.is_none() {
            return Err(message("known coverage requires actual scope"));
        }
        let payload = serde_json::to_string(&input)?;
        let mut conn = store::open_session(&store::session_db_path(self.workspace(), session)?)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(result) = cached(&tx, request, "check", &payload)? {
            return Ok(result);
        }
        claim(&tx, input.target)?;
        sources(&tx, &input.sources)?;
        let id = event(&tx, "check", &payload)?;
        tx.execute(
            "INSERT INTO checks VALUES (?1,?2,?3)",
            params![id, input.target, payload],
        )?;
        let result = Record {
            event: id,
            data: input,
        };
        remember(&tx, request, "check", &payload, &result)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn argument_add(
        &self,
        session: &str,
        request: &str,
        input: ArgumentInput,
    ) -> Result<Record<ArgumentInput>> {
        if input.premises.is_empty() {
            return Err(message("argument requires necessary premises"));
        }
        let payload = serde_json::to_string(&input)?;
        let mut conn = store::open_session(&store::session_db_path(self.workspace(), session)?)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(result) = cached(&tx, request, "argument", &payload)? {
            return Ok(result);
        }
        claim(&tx, input.target)?;
        sources(&tx, &input.sources)?;
        for p in &input.premises {
            claim(&tx, *p)?;
        }
        let id = event(&tx, "argument", &payload)?;
        tx.execute(
            "INSERT INTO arguments VALUES (?1,?2,?3)",
            params![id, input.target, payload],
        )?;
        graph(&tx, &[input.target])?; // Detect a cycle before any event becomes durable.
        let result = Record {
            event: id,
            data: input,
        };
        remember(&tx, request, "argument", &payload, &result)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn claim_show(&self, session: &str, version: i64) -> Result<ClaimView> {
        let mut conn = store::read_session(self.workspace(), session)?
            .ok_or_else(|| message("missing source session"))?;
        let tx = conn.transaction()?;
        view(&tx, version)
    }
    pub fn claim_explain(&self, session: &str, version: i64) -> Result<ClaimGraph> {
        self.claims_explain(session, &[version])
    }

    /// Explain several roots in one consistent read transaction.
    pub fn claims_explain(&self, session: &str, versions: &[i64]) -> Result<ClaimGraph> {
        let mut conn = store::read_session(self.workspace(), session)?
            .ok_or_else(|| message("missing source session"))?;
        let tx = conn.transaction()?;
        graph(&tx, versions)
    }
    pub fn handoff_export(
        &self,
        session: &str,
        request: &str,
        roots: &[i64],
    ) -> Result<ExportedHandoff> {
        let payload = serde_json::to_string(roots)?;
        let mut conn = store::open_session(&store::session_db_path(self.workspace(), session)?)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(result) = cached(&tx, request, "export", &payload)? {
            return Ok(result);
        }
        let manifest = Handoff {
            format: 1,
            workspace: self
                .workspace()
                .canonicalize()?
                .to_string_lossy()
                .into_owned(),
            source_session: session.into(),
            source_position: position(&tx)?,
            graph: graph(&tx, roots)?,
        };
        let id = event(&tx, "handoff_export", &serde_json::to_string(&manifest)?)?;
        let result = ExportedHandoff {
            reference: HandoffRef {
                source_session: session.into(),
                event: id,
            },
            manifest,
        };
        remember(&tx, request, "export", &payload, &result)?;
        tx.commit()?;
        Ok(result)
    }
    /// Permission comes from the trusted host, never from the manifest. A receipt
    /// retains source-qualified references; it does not turn imported prose into evidence.
    pub fn handoff_restore(
        &self,
        target: &str,
        request: &str,
        reference: HandoffRef,
        allowed_sources: &[String],
    ) -> Result<RestoredHandoff> {
        validate_id(target, "target session")?;
        if target == reference.source_session {
            return Err(message("handoff requires a different target session"));
        }
        if !allowed_sources.contains(&reference.source_session) {
            return Err(message("source session is not authorized"));
        }
        let source_path = store::session_db_path(self.workspace(), &reference.source_session)?;
        if !source_path.try_exists()? {
            return Err(message("missing source session"));
        }
        // Pin the source by reserving its writer lock. Concurrent writers either
        // precede this transaction or wait until the receipt commits. Busy is retryable.
        let mut origin =
            Connection::open_with_flags(&source_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        origin.busy_timeout(std::time::Duration::from_millis(100))?;
        let snapshot = origin
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| message(format!("retryable source snapshot unavailable: {e}")))?;
        let raw: String = snapshot
            .query_row(
                "SELECT body FROM events WHERE id=?1 AND event_kind='handoff_export'",
                [reference.event],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| message("missing handoff export"))?;
        let manifest: Handoff = serde_json::from_str(&raw)?;
        if manifest.format != 1
            || manifest.source_session != reference.source_session
            || manifest.workspace != self.workspace().canonicalize()?.to_string_lossy()
        {
            return Err(message("handoff workspace or source mismatch"));
        }
        for v in manifest.graph.claims.values() {
            if claim(&snapshot, v.claim.event)? != v.claim {
                return Err(message("claim content mismatch"));
            }
            let live = view(&snapshot, v.claim.event)?;
            if !v.checks.iter().all(|c| live.checks.contains(c))
                || !v.arguments.iter().all(|a| live.arguments.contains(a))
            {
                return Err(message("evidence record content mismatch"));
            }
            for s in &v.sources {
                if source(&snapshot, s.event)? != *s {
                    return Err(message("source content mismatch"));
                }
            }
        }
        let current = graph(&snapshot, &manifest.graph.roots)?;
        let checked_position = position(&snapshot)?;
        let payload = serde_json::to_string(&reference)?;
        let mut destination =
            store::open_session(&store::session_db_path(self.workspace(), target)?)?;
        let tx = destination.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(result) = cached(&tx, request, "restore", &payload)? {
            return Ok(result);
        }
        let changes = current
            .states
            .iter()
            .filter_map(|(id, after)| {
                manifest
                    .graph
                    .states
                    .get(id)
                    .filter(|before| *before != after)
                    .map(|before| StateChange {
                        version: *id,
                        before: before.clone(),
                        after: after.clone(),
                        reason: "source checks or necessary premise states changed since export"
                            .into(),
                    })
            })
            .collect();
        let receipt_event = event(&tx, "handoff_restore", &payload)?;
        let result = RestoredHandoff {
            reference,
            receipt_event,
            exported_position: manifest.source_position,
            checked_position,
            exported: manifest.graph,
            current,
            changes,
        };
        remember(&tx, request, "restore", &payload, &result)?;
        tx.commit()?;
        Ok(result)
    }
}
