//! Per-task handoffs. Each field update and its WAL event commit together.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::error::{message, Result};
use crate::redact::redact;
use crate::store::{
    current_turn, now_ms, open_session, read_session, session_db_path, validate_id,
};
use crate::{Engine, Note};

#[derive(Debug, Serialize)]
pub struct Hint {
    pub event_id: i64,
    pub text: String,
}

#[derive(Default, Debug, Serialize)]
pub struct TaskPatch {
    pub goal: Option<String>,
    pub progress: Option<String>,
    pub next_step: Option<String>,
    pub pin: Option<Hint>,
    pub unpin: Option<i64>,
    /// A compaction started at this active checkpoint. Zero means no task existed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_active_event: Option<i64>,
    /// Source snapshot of a successfully completed compaction (not its completion time).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacted_through_turn: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TaskState {
    pub task_id: String,
    pub goal: String,
    pub progress: String,
    pub next_step: String,
    pub changed_event: i64,
    pub hints: Vec<Hint>,
}

#[derive(Debug, Serialize)]
pub struct TaskContext {
    pub current_turn: i64,
    pub compacted_through_turn: Option<i64>,
    pub turns_since_compaction: Option<i64>,
    pub task: Option<TaskState>,
    pub standing: Vec<Note>,
    pub preferences: Vec<Note>,
}

impl Engine {
    pub fn task_context(&self, session: &str, task: Option<&str>) -> Result<TaskContext> {
        if let Some(id) = task {
            validate_id(id, "task id")?;
        }
        let (state, turn, through) = match read_session(self.workspace(), session)? {
            Some(mut conn) => {
                let tx = conn.transaction()?;
                let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
                let state = if version == 0 {
                    None
                } else {
                    read_task(&tx, task)?
                };
                let (turn, through) = if version >= 2 {
                    let through = match &state {
                        Some(s) => compaction_turn(&tx, &s.task_id)?,
                        None => None,
                    };
                    (current_turn(&tx)?, through)
                } else {
                    (0, None)
                };
                (state, turn, through)
            }
            None => (None, 0, None),
        };
        Ok(TaskContext {
            current_turn: turn,
            compacted_through_turn: through,
            turns_since_compaction: through.map(|t| turn - t),
            task: state,
            standing: self.standing()?,
            preferences: self.preferences()?,
        })
    }

    /// Select or resume a task. Missing fields survive; an empty string clears a field.
    pub fn update_task(
        &self,
        session: &str,
        task: &str,
        mut patch: TaskPatch,
    ) -> Result<TaskState> {
        validate_id(task, "task id")?;
        let mut changed_by_redaction = false;
        for field in [&mut patch.goal, &mut patch.progress, &mut patch.next_step]
            .into_iter()
            .flatten()
        {
            *field = compact_text(field, 2048, &mut changed_by_redaction)?;
        }
        if let Some(pin) = &mut patch.pin {
            pin.text = compact_text(&pin.text, 320, &mut changed_by_redaction)?;
            if pin.text.is_empty() || Some(pin.event_id) == patch.unpin {
                return Err(message("pin needs a short hint and cannot also be removed"));
            }
        }
        let mut conn = open_session(&session_db_path(self.workspace(), session)?)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active = read_task(&tx, None)?;
        if let Some(expected) = patch.if_active_event {
            let matches = match &active {
                Some(state) => state.task_id == task && state.changed_event == expected,
                None => expected == 0,
            };
            if !matches {
                return Err(message("active task changed since checkpoint"));
            }
        }
        let refresh = if let Some(through) = patch.compacted_through_turn {
            let previous = compaction_turn(&tx, task)?;
            if patch.if_active_event.is_none()
                || !(0..=current_turn(&tx)?).contains(&through)
                || previous.is_some_and(|old| through < old)
            {
                return Err(message("invalid or stale compaction turn"));
            }
            previous != Some(through)
        } else {
            false
        };
        if let Some(state) = active {
            let changed = patch.goal.as_ref().is_some_and(|v| v != &state.goal)
                || patch
                    .progress
                    .as_ref()
                    .is_some_and(|v| v != &state.progress)
                || patch
                    .next_step
                    .as_ref()
                    .is_some_and(|v| v != &state.next_step)
                || patch.pin.as_ref().is_some_and(|new| {
                    !state
                        .hints
                        .iter()
                        .any(|h| h.event_id == new.event_id && h.text == new.text)
                })
                || patch
                    .unpin
                    .is_some_and(|id| state.hints.iter().any(|h| h.event_id == id));
            if state.task_id == task && !changed && !refresh {
                return Ok(state);
            }
        }
        if let Some(pin) = &patch.pin {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM events WHERE id=?1)",
                [pin.event_id],
                |r| r.get(0),
            )?;
            if !exists {
                return Err(message("hint source does not exist in this session"));
            }
        }
        tx.execute(
            "INSERT INTO events(task_id, event_kind, body, redacted, created_at)
             VALUES (?1, 'task_checkpoint', ?2, ?3, ?4)",
            params![
                task,
                serde_json::to_string(&patch)?,
                changed_by_redaction,
                now_ms()
            ],
        )?;
        let event = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO tasks(task_id, goal, progress, next_step, changed_event)
             VALUES (?1, COALESCE(?2,''), COALESCE(?3,''), COALESCE(?4,''), ?5)
             ON CONFLICT(task_id) DO UPDATE SET goal=COALESCE(?2,goal), progress=COALESCE(?3,progress),
             next_step=COALESCE(?4,next_step), changed_event=?5",
            params![task, patch.goal, patch.progress, patch.next_step, event],
        )?;
        if let Some(turn) = patch.compacted_through_turn {
            tx.execute(
                "INSERT INTO task_compactions(task_id, through_turn) VALUES (?1,?2)
                ON CONFLICT(task_id) DO UPDATE SET through_turn=excluded.through_turn",
                params![task, turn],
            )?;
        }
        if let Some(id) = patch.unpin {
            tx.execute(
                "DELETE FROM task_hints WHERE task_id=?1 AND event_id=?2",
                params![task, id],
            )?;
        }
        if let Some(hint) = patch.pin {
            tx.execute("INSERT INTO task_hints VALUES (?1,?2,?3) ON CONFLICT(task_id,event_id) DO UPDATE SET hint=excluded.hint",
                       params![task,hint.event_id,hint.text])?;
            let count: usize = tx.query_row(
                "SELECT COUNT(*) FROM task_hints WHERE task_id=?1",
                [task],
                |r| r.get(0),
            )?;
            if count > 12 {
                return Err(message(
                    "task already has 12 hints; remove a resolved hint first",
                ));
            }
        }
        let state =
            read_task(&tx, Some(task))?.ok_or_else(|| message("missing task after update"))?;
        tx.commit()?;
        Ok(state)
    }
}

fn compaction_turn(conn: &Connection, task: &str) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT through_turn FROM task_compactions WHERE task_id=?1",
            [task],
            |row| row.get(0),
        )
        .optional()?)
}

fn compact_text(text: &str, max: usize, changed: &mut bool) -> Result<String> {
    if text.len() > max {
        return Err(message(format!("task text exceeds {max} bytes")));
    }
    let (clean, redacted) = redact(text.trim());
    *changed |= redacted;
    Ok(clean)
}

fn read_task(conn: &Connection, task: Option<&str>) -> Result<Option<TaskState>> {
    let state = conn.query_row(
        "SELECT task_id, goal, progress, next_step, changed_event FROM tasks
         WHERE task_id=COALESCE(?1,(SELECT task_id FROM tasks ORDER BY changed_event DESC LIMIT 1))",
        [task], |row| Ok(TaskState {
            task_id: row.get(0)?, goal: row.get(1)?, progress: row.get(2)?,
            next_step: row.get(3)?, changed_event: row.get(4)?, hints: Vec::new(),
        }),
    ).optional()?;
    let Some(mut state) = state else {
        return Ok(None);
    };
    let mut query =
        conn.prepare("SELECT event_id, hint FROM task_hints WHERE task_id=?1 ORDER BY event_id")?;
    state.hints = query
        .query_map([&state.task_id], |r| {
            Ok(Hint {
                event_id: r.get(0)?,
                text: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(state))
}
