//! Read-only evidence for a single named setting at a real user-message boundary.
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::{message, Result};
use crate::store::read_session;
use crate::tags::{contains_term, load};
use crate::Engine;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Vocabulary {
    pub fields: Vec<String>,
    pub scopes: Vec<String>,
    pub periods: Vec<String>,
}

#[derive(Serialize)]
pub struct Original {
    pub id: i64,
    pub event_kind: String,
    pub body: String,
}

#[derive(Serialize)]
pub struct ReviewContext {
    pub question: String,
    pub through_event: i64,
    pub fields: Vec<String>,
    pub scopes: Vec<String>,
    pub periods: Vec<String>,
    pub coverage: &'static str,
    pub candidate_ids: Vec<i64>,
    pub originals: Vec<Original>,
}

fn mentioned(text: &str, names: &[String]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for name in names {
        let name = name.trim();
        let lower = name.to_lowercase();
        if contains_term(text, &lower) && !found.iter().any(|n| n.to_lowercase() == lower) {
            found.push(name.to_owned());
        }
    }
    found
}

impl Engine {
    pub fn review_context(
        &self,
        session: &str,
        question_id: &str,
        vocab: &Vocabulary,
    ) -> Result<ReviewContext> {
        if question_id.is_empty()
            || question_id.len() > 256
            || vocab
                .fields
                .iter()
                .chain(&vocab.scopes)
                .chain(&vocab.periods)
                .any(|n| n.trim().is_empty() || n.chars().count() > 64)
        {
            return Err(message("invalid question ID or workspace vocabulary"));
        }
        let mut conn =
            read_session(self.workspace(), session)?.ok_or_else(|| message("session not found"))?;
        let tx = conn.transaction()?;
        let mut q = tx.prepare(
            "SELECT id,body FROM events
             WHERE actor='harness' AND event_kind='user_message' AND topic=?1
             ORDER BY id LIMIT 2",
        )?;
        let mut rows = q.query([question_id])?;
        let first = rows
            .next()?
            .ok_or_else(|| message("captured user question not found"))?;
        let boundary: i64 = first.get(0)?;
        let question: String = first.get(1)?;
        if rows.next()?.is_some() || question.len() > 4096 {
            return Err(message("question exceeds review budget"));
        }
        let lower = question.to_lowercase();
        let mut ctx = ReviewContext {
            question,
            through_event: boundary - 1,
            fields: mentioned(&lower, &vocab.fields),
            scopes: mentioned(&lower, &vocab.scopes),
            periods: mentioned(&lower, &vocab.periods),
            coverage: "single_target_required",
            candidate_ids: vec![],
            originals: vec![],
        };
        let [field] = ctx.fields.as_slice() else {
            return Ok(ctx);
        };
        if ctx.scopes.len() > 1 || ctx.periods.len() > 1 {
            return Ok(ctx);
        }
        let field = field.to_lowercase();
        let scope = ctx.scopes.first().map(|x| x.to_lowercase());
        let period = ctx.periods.first().map(|x| x.to_lowercase());
        ctx.coverage = "named_mentions";
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        // All validated tag literals occur in the body. The raw pass also keeps
        // untagged denials/revisions and matches embedded identifiers in CJK text.
        let mut stmt = tx.prepare(
            "SELECT id,event_kind,body FROM events
            WHERE id<?1 AND actor='harness' AND substr(event_kind,1,5)!='call.'
            AND substr(event_kind,1,7)!='memory_' AND event_kind!='session_config'
            AND (instr(lower(body),lower(?2)) OR
                 (instr(lower(body),lower(?3)) AND instr(lower(body),lower(?4)))) ORDER BY id",
        )?;
        // SQLite lower() only case-folds ASCII. Non-ASCII names use the Rust check below.
        let prefilter = |name: &str| {
            if name.is_ascii() {
                name.to_owned()
            } else {
                String::new()
            }
        };
        let mut rows = stmt.query(params![
            boundary,
            prefilter(&field),
            scope.as_deref().map(prefilter),
            period.as_deref().map(prefilter)
        ])?;
        let mut bytes = 0;
        while let Some(r) = rows.next()? {
            let source = Original {
                id: r.get(0)?,
                event_kind: r.get(1)?,
                body: r.get(2)?,
            };
            let body = source.body.to_lowercase();
            let dimensions = scope
                .as_ref()
                .zip(period.as_ref())
                .is_some_and(|(s, p)| contains_term(&body, s) && contains_term(&body, p));
            if !contains_term(&body, &field) && !dimensions {
                continue;
            }
            bytes += source.body.len();
            if ctx.originals.len() == 16 || bytes > 4096 {
                ctx.coverage = "partial";
                break;
            }
            if version >= 3 {
                if let Some(tags) = load(&tx, source.id)? {
                    let matches = |text: Option<&str>, name: &Option<String>| {
                        name.as_ref().is_none_or(|n| {
                            text.is_some_and(|s| contains_term(&s.to_lowercase(), n))
                        })
                    };
                    if tags.facts.iter().any(|f| {
                        contains_term(&f.field.to_lowercase(), &field)
                            && matches(f.scope.as_deref(), &scope)
                            && matches(f.period.as_deref(), &period)
                    }) {
                        ctx.candidate_ids.push(source.id);
                    }
                }
            }
            ctx.originals.push(source);
        }
        Ok(ctx)
    }
}
