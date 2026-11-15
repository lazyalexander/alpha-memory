//! Optional, source-anchored model annotations on immutable captured events.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::error::{message, Result};
use crate::store::{open_session, session_db_path};
use crate::Engine;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FactKind {
    Decision,
    Observation,
    Scratch,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FactTag {
    pub kind: FactKind,
    pub field: String,
    pub scope: Option<String>,
    pub period: Option<String>,
    pub value: String,
    /// The old literal named in the quote. This does not establish an event-to-event link.
    pub replaces: Option<String>,
    pub quote: String,
}

#[derive(Debug, Serialize)]
pub struct InferredTags {
    pub extractor: String,
    pub facts: Vec<FactTag>,
}

pub(crate) fn contains_term(quote: &str, term: &str) -> bool {
    // Avoid accepting a different identifier or number (e.g. "44" inside "448").
    // CJK characters need not be separated from a literal by whitespace.
    let word = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-');
    let joins = |a: Option<char>, b: Option<char>| a.is_some_and(word) && b.is_some_and(word);
    !term.is_empty()
        && quote.match_indices(term).any(|(i, _)| {
            !joins(quote[..i].chars().next_back(), term.chars().next())
                && !joins(
                    term.chars().next_back(),
                    quote[i + term.len()..].chars().next(),
                )
        })
}

impl FactTag {
    fn supported_by(&self, body: &str) -> bool {
        self.quote.len() <= 4096
            && contains_term(body, &self.quote)
            && [
                Some(&self.field),
                self.scope.as_ref(),
                self.period.as_ref(),
                Some(&self.value),
                self.replaces.as_ref(),
            ]
            .into_iter()
            .flatten()
            .all(|s| {
                s.trim() == s
                    && s.len() <= 256
                    && !s.contains("[REDACTED]")
                    && contains_term(&self.quote, s)
            })
    }
}

impl Engine {
    /// Commit annotations separately from capture. A failed attempt leaves the event available
    /// for retry. A successful empty array records an abstention. First successful write wins.
    pub fn tag_event(
        &self,
        session: &str,
        event: i64,
        extractor: &str,
        facts: &[FactTag],
    ) -> Result<bool> {
        let path = session_db_path(self.workspace(), session)?;
        if event <= 0 || !path.try_exists()? {
            return Err(message("captured event not found"));
        }
        if extractor.is_empty()
            || extractor.len() > 160
            || !extractor
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-:/".contains(&b))
        {
            return Err(message("invalid extractor identifier"));
        }
        if facts.len() > 8 {
            return Err(message("at most eight facts per event"));
        }
        let mut conn = open_session(&path)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let body: String = tx
            .query_row(
                "SELECT body FROM events WHERE id=?1 AND actor='harness'",
                [event],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| message("captured event not found"))?;
        if !facts.iter().all(|f| f.supported_by(&body)) {
            return Err(message(
                "facts must quote their own stored event and anchor each literal in that quote",
            ));
        }
        let changed = tx.execute(
            "INSERT INTO event_fact_tags(event_id,extractor,facts) VALUES (?1,?2,?3) ON CONFLICT(event_id) DO NOTHING",
            params![event, extractor, serde_json::to_string(facts)?],
        )?;
        tx.commit()?;
        Ok(changed == 1)
    }
}

pub(crate) fn load(conn: &Connection, event: i64) -> Result<Option<InferredTags>> {
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT extractor,facts FROM event_fact_tags WHERE event_id=?1",
            [event],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(extractor, json)| {
        Ok(InferredTags {
            extractor,
            facts: serde_json::from_str(&json)?,
        })
    })
    .transpose()
}
