use alpha_memory::{Append, Engine, RecallQuery, ReflectLong, ReflectMid};

fn engine() -> (tempfile::TempDir, tempfile::TempDir, Engine) {
    let workspace = tempfile::tempdir().unwrap();
    let user_home = tempfile::tempdir().unwrap();
    let engine = Engine::open(workspace.path(), user_home.path()).unwrap();
    (workspace, user_home, engine)
}

fn append(engine: &Engine, session: &str, body: &str) -> i64 {
    engine
        .append(Append {
            session_id: session.into(),
            task_id: None,
            actor: None,
            event_kind: None,
            body: body.into(),
            config_key: None,
            config_value: None,
            stage: None,
            topic: None,
            artifact_path: None,
            artifact_hash: None,
            artifact_size: None,
            failure: false,
            decision: false,
        })
        .unwrap()
        .event_id
}

fn recall(engine: &Engine, session: &str, open_long_term: bool) -> alpha_memory::RecallPack {
    engine
        .recall(RecallQuery {
            session_id: session.into(),
            task_id: None,
            open_long_term,
            stage: None,
            recent: alpha_memory::DEFAULT_RECENT,
        })
        .unwrap()
}

#[test]
fn user_notes_are_always_recalled_and_stay_out_of_the_session_log() {
    let (_workspace, _home, engine) = engine();
    engine.add_standing("Do not drop the database.").unwrap();
    engine.add_preference("Prefer small patches.").unwrap();
    append(&engine, "s1", "hello");
    append(&engine, "s2", "other session");

    for session in ["s1", "s2"] {
        let pack = recall(&engine, session, false);
        assert_eq!(pack.standing[0].text, "Do not drop the database.");
        assert_eq!(pack.preferences[0].text, "Prefer small patches.");
        assert!(pack.long_term.is_empty());
        assert!(!pack.long_term_opened);
    }

    let events = engine.list_events("s1").unwrap();
    assert!(events
        .iter()
        .all(|event| event.body != "Prefer small patches."));
    assert!(events
        .iter()
        .all(|event| event.body != "Do not drop the database."));
}

#[test]
fn sessions_do_not_share_config_decisions_or_failures() {
    let (_workspace, _home, engine) = engine();
    engine
        .append(Append {
            session_id: "s1".into(),
            body: "Use SQLite.".into(),
            decision: true,
            topic: Some("db".into()),
            ..empty("s1")
        })
        .unwrap();
    engine
        .append(Append {
            session_id: "s1".into(),
            config_key: Some("port".into()),
            config_value: Some("4080".into()),
            ..empty("s1")
        })
        .unwrap();
    let pack = recall(&engine, "s2", true);
    assert!(pack.recent_decisions.is_empty());
    assert!(pack.config.is_empty());
    assert!(pack.long_term.is_empty());
}

#[test]
fn recent_config_keys_are_left_out_until_they_age_out() {
    let (_workspace, _home, engine) = engine();
    engine
        .append(Append {
            config_key: Some("port".into()),
            config_value: Some("4080".into()),
            ..empty("s1")
        })
        .unwrap();
    for index in 0..5 {
        append(&engine, "s1", &format!("step {index}"));
    }
    let visible = recall(&engine, "s1", false);
    assert_eq!(visible.config[0].value, "4080");

    engine
        .append(Append {
            config_key: Some("port".into()),
            config_value: Some("5080".into()),
            ..empty("s1")
        })
        .unwrap();
    let hidden = recall(&engine, "s1", false);
    assert!(hidden.config.is_empty());
}

#[test]
fn long_term_stays_behind_the_caller_gate() {
    let (_workspace, _home, engine) = engine();
    let event_id = engine
        .append(Append {
            body: "Use SQLite for the session log.".into(),
            decision: true,
            topic: Some("db".into()),
            ..empty("s1")
        })
        .unwrap()
        .event_id;
    let mid = engine
        .reflect_mid(ReflectMid {
            session_id: "s1".into(),
            task_id: None,
            start_event_id: event_id,
            end_event_id: event_id,
            body: Some("Session log is SQLite.".into()),
            topic: Some("db".into()),
        })
        .unwrap();
    let closed = recall(&engine, "s1", false);
    assert_eq!(closed.mid_term.len(), 1);
    assert!(closed.long_term.is_empty());
    assert!(closed.recent_decisions.is_empty());

    engine
        .reflect_long(ReflectLong {
            session_id: "s1".into(),
            mid_ids: vec![mid.id],
            body: Some("The session log stays in SQLite.".into()),
            topic: Some("db".into()),
        })
        .unwrap();
    let still_closed = recall(&engine, "s1", false);
    assert!(still_closed.mid_term.is_empty());
    assert!(still_closed.long_term.is_empty());

    let opened = recall(&engine, "s1", true);
    assert!(opened.mid_term.is_empty());
    assert_eq!(opened.long_term.len(), 1);
    assert_eq!(opened.long_term[0].mid_ids, vec![mid.id]);
}

#[test]
fn a_new_decision_supersedes_the_old_entry() {
    let (_workspace, _home, engine) = engine();
    let first = append(&engine, "s1", "note");
    engine
        .reflect_mid(ReflectMid {
            session_id: "s1".into(),
            task_id: None,
            start_event_id: first,
            end_event_id: first,
            body: Some("Use Postgres.".into()),
            topic: Some("db".into()),
        })
        .unwrap();
    engine
        .append(Append {
            body: "Use SQLite.".into(),
            decision: true,
            topic: Some("db".into()),
            ..empty("s1")
        })
        .unwrap();
    let pack = recall(&engine, "s1", false);
    assert!(pack.mid_term.is_empty());
    assert_eq!(pack.recent_decisions.len(), 1);
    assert_eq!(pack.recent_decisions[0].body, "Use SQLite.");
}

#[test]
fn secrets_are_redacted_before_the_log_is_durable() {
    let (_workspace, _home, engine) = engine();
    let appended = engine
        .append(Append {
            body: "token=supersecret".into(),
            config_key: Some("api_key".into()),
            config_value: Some("sk-abcdefghijklmnopqrstuvwxyz".into()),
            ..empty("s1")
        })
        .unwrap();
    assert!(appended.redacted);
    let event = &engine.list_events("s1").unwrap()[0];
    assert_eq!(event.body, "token=[REDACTED]");
    assert_eq!(event.config_value.as_deref(), Some("[REDACTED]"));
    assert!(!event.body.contains("supersecret"));
}

#[test]
fn recall_prefers_the_current_stage() {
    let (_workspace, _home, engine) = engine();
    engine
        .append(Append {
            stage: Some("exploring".into()),
            body: "skip tests while exploring".into(),
            ..empty("s1")
        })
        .unwrap();
    engine
        .append(Append {
            stage: Some("hardening".into()),
            body: "run the full suite".into(),
            ..empty("s1")
        })
        .unwrap();
    let closed = recall(&engine, "s1", false);
    assert_eq!(closed.stage.unwrap().stage.as_deref(), Some("hardening"));
    assert!(closed.env.is_empty());

    let opened = recall(&engine, "s1", true);
    assert!(opened
        .env
        .iter()
        .all(|event| event.stage.as_deref() != Some("exploring")));
    assert!(opened.env.is_empty());
}

#[test]
fn artifacts_stay_structured() {
    let (_workspace, _home, engine) = engine();
    engine
        .append(Append {
            artifact_path: Some("src/lib.rs".into()),
            artifact_hash: Some("abc123".into()),
            artifact_size: Some(12),
            ..empty("s1")
        })
        .unwrap();
    let pack = recall(&engine, "s1", false);
    assert_eq!(pack.artifacts[0].path, "src/lib.rs");
    assert_eq!(pack.artifacts[0].hash, "abc123");
    assert_eq!(pack.artifacts[0].size, Some(12));
}

#[test]
fn failure_and_decision_together_are_rejected() {
    let (_workspace, _home, engine) = engine();
    let error = engine
        .append(Append {
            body: "nope".into(),
            failure: true,
            decision: true,
            ..empty("s1")
        })
        .unwrap_err();
    assert!(error.to_string().contains("either a failure or a decision"));
}

fn empty(session: &str) -> Append {
    Append {
        session_id: session.into(),
        task_id: None,
        actor: None,
        event_kind: None,
        body: String::new(),
        config_key: None,
        config_value: None,
        stage: None,
        topic: None,
        artifact_path: None,
        artifact_hash: None,
        artifact_size: None,
        failure: false,
        decision: false,
    }
}

#[test]
fn task_handoffs_resume_without_history_and_failed_pins_are_atomic() {
    use alpha_memory::{Hint, TaskPatch};
    let (ws, home, eng) = engine();
    let session = "handoff";
    let conf = eng
        .append(Append {
            config_key: Some("staging.port".into()),
            config_value: Some(" 04761 ".into()),
            body: "Historical explanation.".into(),
            ..empty(session)
        })
        .unwrap();
    eng.update_task(
        session,
        "archive",
        TaskPatch {
            goal: Some("Maintain the probe".into()),
            pin: Some(Hint {
                event_id: conf.event_id,
                text: "Verify archive compatibility".into(),
            }),
            ..Default::default()
        },
    )
    .unwrap();
    eng.update_task(
        session,
        "archive",
        TaskPatch {
            progress: Some("token=hidden".into()),
            ..Default::default()
        },
    )
    .unwrap();
    eng.update_task(
        session,
        "main",
        TaskPatch {
            next_step: Some("Run CI".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let restored = Engine::open(ws.path(), home.path()).unwrap();
    let current = restored.task_context(session, None).unwrap();
    assert_eq!(current.task.as_ref().unwrap().task_id, "main");
    let compact = serde_json::to_string(&current).unwrap();
    for old in ["archive", "Historical explanation", "04761", "staging.port"] {
        assert!(!compact.contains(old));
    }
    let count = restored.list_events(session).unwrap().len();
    assert!(restored
        .update_task(
            session,
            "archive",
            TaskPatch {
                goal: Some("uncommitted".into()),
                pin: Some(Hint {
                    event_id: 999,
                    text: "invalid".into()
                }),
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(restored.list_events(session).unwrap().len(), count);
    assert_eq!(
        restored
            .task_context(session, None)
            .unwrap()
            .task
            .unwrap()
            .task_id,
        "main"
    );
    let state = restored
        .update_task(session, "archive", TaskPatch::default())
        .unwrap();
    assert_eq!(state.goal, "Maintain the probe");
    assert_eq!(state.progress, "token=[REDACTED]");
    assert_eq!(state.hints[0].event_id, conf.event_id);
    assert!(restored
        .update_task(
            session,
            "archive",
            TaskPatch {
                unpin: Some(conf.event_id),
                ..Default::default()
            }
        )
        .unwrap()
        .hints
        .is_empty());
    let current_val = restored
        .session_config(session, "staging.port")
        .unwrap()
        .unwrap();
    assert_eq!(current_val.value, " 04761 ");
    assert_eq!(current_val.event_id, conf.event_id);
    assert!(restored
        .session_config(session, "production.port")
        .unwrap()
        .is_none());
    assert!(restored
        .session_config("other", "staging.port")
        .unwrap()
        .is_none());
    assert!(restored.task_context("other", None).unwrap().task.is_none());
}

#[test]
fn search_reads_originals_after_updates_and_before_lazy_upgrade() {
    use alpha_memory::TaskPatch;
    use rusqlite::Connection;
    let (ws, _, eng) = engine();
    let first = eng
        .append(Append {
            body: "旧版探针使用前一套协议".into(),
            topic: Some("probe-cutover".into()),
            config_key: Some("staging.port".into()),
            config_value: Some("4761".into()),
            ..empty("s")
        })
        .unwrap()
        .event_id;
    let last = eng
        .append(Append {
            body: "Rotated the probe; token=classified".into(),
            config_key: Some("staging.port".into()),
            config_value: Some("8327".into()),
            ..empty("s")
        })
        .unwrap()
        .event_id;
    // Emulate the previous schema, which has all the original event and slot rows.
    let db = ws.path().join(".alpha-memory/sessions/s/store.sqlite");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("DROP TABLE claim_requests; DROP TABLE arguments; DROP TABLE checks; DROP TABLE claims; DROP TABLE event_fact_tags; DROP TABLE user_turns; DROP TABLE task_compactions; DROP TRIGGER index_appended_event; DROP TABLE event_index; DROP TABLE tasks; DROP TABLE task_hints; PRAGMA user_version=0;").unwrap();
    assert!(eng.task_context("s", None).unwrap().task.is_none());
    assert_eq!(
        eng.evidence_index("s", "旧版探针", None, None, 6).unwrap()[0].event_id,
        first
    );
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    eng.update_task("s", "migrate", TaskPatch::default())
        .unwrap();
    assert_eq!(
        eng.evidence_index("s", "probe-cutover", None, Some(last), 6)
            .unwrap()[0]
            .event_id,
        first
    );
    assert!(eng
        .evidence_index("s", "classified", None, None, 6)
        .unwrap()
        .is_empty());
    assert!(eng
        .evidence_index("s", "probe-cutover", Some("migrate"), None, 6)
        .unwrap()
        .is_empty());
    assert!(eng
        .evidence_index("s", "旧版探针", None, None, 6)
        .unwrap()
        .iter()
        .any(|e| e.event_id == first));
    let history = eng.source("s", first, 1).unwrap();
    assert_eq!(history[0].next_config_event, Some(last));
    assert_eq!(history[1].previous_config_event, Some(first));
    assert_eq!(history[0].event.config_value.as_deref(), Some("4761"));
    assert_eq!(history[1].event.config_value.as_deref(), Some("8327"));
    assert_eq!(
        eng.session_config("s", "staging.port")
            .unwrap()
            .unwrap()
            .event_id,
        last
    );
    assert!(eng.source("s", 999, 3).is_err());
    assert!(eng.source("other", first, 0).is_err());
    assert!(eng.source("../s", first, 0).is_err());
    eng.append(Append {
        body: "Search me with needleterm".into(),
        ..empty("s")
    })
    .unwrap();
    assert_eq!(
        eng.evidence_index("s", "needleterm", Some("migrate"), None, 3)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn capture_redacts_before_splitting_and_retries_do_not_duplicate_evidence() {
    let (_workspace, _home, engine) = engine();
    let text = format!(
        "{}token=sensitive\n{}",
        "a ".repeat(131070),
        "尾".repeat(90)
    );
    let ids = engine.capture("s", "tool", Some("call-1"), &text).unwrap();
    assert!(ids.len() > 1);
    assert_eq!(
        engine.capture("s", "tool", Some("call-1"), &text).unwrap(),
        ids
    );
    assert!(engine
        .capture("s", "tool", Some("call-1"), "different")
        .is_err());
    let events = engine.list_events("s").unwrap();
    let joined = events.iter().map(|e| e.body.as_str()).collect::<String>();
    assert_eq!(joined, text.replace("sensitive", "[REDACTED]"));
    assert_eq!(events.len(), ids.len());
}

#[test]
fn compaction_respects_holes_tasks_and_previously_promoted_spans() {
    use alpha_memory::TaskPatch;
    let (_ws, _home, e) = engine();
    e.update_task("s", "first", TaskPatch::default()).unwrap();
    let covered = append(&e, "s", "old observation");
    let mid = e
        .reflect_mid(ReflectMid {
            session_id: "s".into(),
            task_id: Some("first".into()),
            start_event_id: covered,
            end_event_id: covered,
            body: Some("old summary".into()),
            topic: None,
        })
        .unwrap();
    e.reflect_long(ReflectLong {
        session_id: "s".into(),
        mid_ids: vec![mid.id],
        body: Some("retained".into()),
        topic: None,
    })
    .unwrap();
    e.update_task("s", "second", TaskPatch::default()).unwrap();
    append(&e, "s", "new observation");
    let spans = e.reflect_pending("s").unwrap();
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].task_id.as_deref(), Some("first"));
    assert_eq!(spans[1].task_id.as_deref(), Some("second"));
    assert!(spans[0].end_event_id < covered && spans[1].start_event_id > covered);
    assert!(e.reflect_pending("s").unwrap().is_empty());
    assert_eq!(e.source_span("s", mid.id).unwrap().start_event_id, covered);
    let slot = e
        .append(Append {
            config_key: Some("scope.empty".into()),
            config_value: Some(String::new()),
            ..empty("s")
        })
        .unwrap();
    assert_eq!(slot.kind, Some(alpha_memory::Kind::Config));
    assert_eq!(
        e.session_config("s", "scope.empty").unwrap().unwrap().value,
        ""
    );
}

#[test]
fn turn_age_uses_the_input_snapshot_and_survives_task_switches() {
    use alpha_memory::TaskPatch;
    let (ws, home, e) = engine();
    let id = "clock";
    let state = e.update_task(id, "alpha", TaskPatch::default()).unwrap();
    assert_eq!(
        e.task_context(id, None).unwrap().turns_since_compaction,
        None
    );
    let large = "x".repeat(257 * 1024);
    let chunks = e
        .capture(id, "user_message", Some("request-1"), &large)
        .unwrap();
    assert!(chunks.len() > 1);
    e.capture(id, "user_message", Some("request-1"), &large)
        .unwrap();
    e.capture(id, "assistant_message", Some("reply"), "done")
        .unwrap();
    append(&e, id, "tool evidence");
    let start = e.task_context(id, None).unwrap();
    assert_eq!(start.current_turn, 1);
    // Another user request arrives while the model summarizes the turn-1 snapshot.
    e.capture(id, "user_message", Some("file-only"), "")
        .unwrap();
    e.capture(id, "user_message", Some("file-only"), "")
        .unwrap();
    let reviewed = e
        .update_task(
            id,
            "alpha",
            TaskPatch {
                if_active_event: Some(state.changed_event),
                compacted_through_turn: Some(start.current_turn),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        e.task_context(id, None).unwrap().turns_since_compaction,
        Some(1)
    );
    for invalid in [0, 99] {
        assert!(e
            .update_task(
                id,
                "alpha",
                TaskPatch {
                    if_active_event: Some(reviewed.changed_event),
                    compacted_through_turn: Some(invalid),
                    ..Default::default()
                }
            )
            .is_err());
    }
    e.update_task(id, "beta", TaskPatch::default()).unwrap();
    e.append(Append {
        event_kind: Some("user_message".into()),
        body: "continue beta".into(),
        ..empty(id)
    })
    .unwrap();
    assert_eq!(
        e.task_context(id, None).unwrap().turns_since_compaction,
        None
    );
    assert!(e
        .update_task(
            id,
            "alpha",
            TaskPatch {
                if_active_event: Some(reviewed.changed_event),
                compacted_through_turn: Some(2),
                ..Default::default()
            }
        )
        .is_err());
    let resumed = e
        .update_task(
            id,
            "alpha",
            TaskPatch {
                progress: Some("updated manually".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let reopened = Engine::open(ws.path(), home.path()).unwrap();
    let before = reopened.task_context(id, None).unwrap();
    assert_eq!(
        (
            before.current_turn,
            before.compacted_through_turn,
            before.turns_since_compaction
        ),
        (3, Some(1), Some(2))
    );
    reopened
        .update_task(
            id,
            "alpha",
            TaskPatch {
                if_active_event: Some(resumed.changed_event),
                compacted_through_turn: Some(3),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        reopened
            .task_context(id, None)
            .unwrap()
            .turns_since_compaction,
        Some(0)
    );
    assert_eq!(
        reopened.task_context("other", None).unwrap().current_turn,
        0
    );
}

#[test]
fn old_handoffs_have_no_invented_compaction_turn() {
    use alpha_memory::TaskPatch;
    use rusqlite::Connection;
    let (ws, _home, e) = engine();
    e.update_task(
        "legacy",
        "task",
        TaskPatch {
            goal: Some("persist".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let db =
        Connection::open(ws.path().join(".alpha-memory/sessions/legacy/store.sqlite")).unwrap();
    db.execute_batch("DROP TABLE claim_requests; DROP TABLE arguments; DROP TABLE checks; DROP TABLE claims; DROP TABLE event_fact_tags; DROP TABLE user_turns; DROP TABLE task_compactions; PRAGMA user_version=1;")
        .unwrap();
    let old = e.task_context("legacy", None).unwrap();
    assert_eq!(old.task.unwrap().goal, "persist");
    assert_eq!(old.turns_since_compaction, None);
    e.capture("legacy", "user_message", Some("new-request"), "continue")
        .unwrap();
    let new = e.task_context("legacy", None).unwrap();
    assert_eq!((new.current_turn, new.compacted_through_turn), (1, None));
}

#[test]
fn inferred_facts_are_checked_against_their_capture_and_never_promoted() {
    use alpha_memory::{FactKind, FactTag};
    use rusqlite::Connection;
    let (ws, _home, eng) = engine();
    let body = "For preview on Cobalt, replace width 448 with 544. token=secret-data";
    let event = eng
        .capture("s", "user_message", Some("part1"), body)
        .unwrap()[0];
    // A read of a v2 capture stays read-only. Its first tag write upgrades it.
    let db = Connection::open(ws.path().join(".alpha-memory/sessions/s/store.sqlite")).unwrap();
    db.execute_batch("DROP TABLE claim_requests; DROP TABLE arguments; DROP TABLE checks; DROP TABLE claims; DROP TABLE event_fact_tags; PRAGMA user_version=2;")
        .unwrap();
    let src = || eng.source("s", event, 0).unwrap().pop().unwrap();
    let raw = src().event.body;
    assert!(!raw.contains("secret-data"));
    assert!(src().inferred_tags.is_none());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    let mut fact = FactTag {
        kind: FactKind::Decision,
        field: "width".into(),
        scope: Some("preview".into()),
        period: Some("Cobalt".into()),
        value: "544".into(),
        replaces: Some("448".into()),
        quote: "For preview on Cobalt, replace width 448 with 544.".into(),
    };
    let write = |f: &FactTag| eng.tag_event("s", event, "trial/v1", std::slice::from_ref(f));
    fact.scope = Some("production".into());
    assert!(write(&fact).is_err());
    fact.scope = Some("preview".into());
    fact.value = "54".into();
    fact.quote = fact.quote.trim_end_matches("4.").into();
    assert!(write(&fact).is_err()); // a partial number cannot be smuggled in a truncated quote
    fact.quote = "For preview on Cobalt, replace width 448 with 544.".into();
    fact.value = "secret-data".into();
    assert!(write(&fact).is_err());
    assert!(src().inferred_tags.is_none());
    fact.value = "544".into();
    assert!(write(&fact).unwrap());
    assert!(!eng.tag_event("s", event, "another-model", &[]).unwrap());
    let saved = src();
    assert_eq!(saved.event.body, raw);
    assert_eq!(saved.event.topic.as_deref(), Some("part1"));
    assert!(saved.event.kind.is_none() && saved.event.stage.is_none());
    assert_eq!(saved.inferred_tags.unwrap().facts[0].value, "544");
    assert!(recall(&eng, "s", false).recent_decisions.is_empty());
    assert!(eng.session_config("s", "width").unwrap().is_none());

    let other = eng
        .capture("s", "assistant_message", Some("part2"), "Only width 544.")
        .unwrap()[0];
    assert!(eng.tag_event("s", other, "trial/v1", &[fact]).is_err());
    assert!(eng.tag_event("s", other, "trial/v1", &[]).unwrap());
    assert!(eng.source("s", other, 0).unwrap()[0]
        .inferred_tags
        .as_ref()
        .unwrap()
        .facts
        .is_empty());
}

#[test]
fn review_uses_the_captured_question_boundary_and_keeps_untagged_counterevidence() {
    use alpha_memory::{FactKind, FactTag, Vocabulary};
    use rusqlite::Connection;
    let (work, _user, engine) = engine();
    let add = |kind: &str, part: &str, text: &str| {
        engine.capture("review", kind, Some(part), text).unwrap()[0]
    };
    let approved = "For north on Juniper we approved port 7400.";
    let first = add("user_message", "old", approved);
    let tag = FactTag {
        kind: FactKind::Decision,
        field: "port".into(),
        scope: Some("north".into()),
        period: Some("Juniper".into()),
        value: "7400".into(),
        replaces: None,
        quote: approved.into(),
    };
    engine.tag_event("review", first, "model", &[tag]).unwrap();
    let denial = add(
        "assistant_message",
        "denied",
        "north explicitly withdrew that approval for Juniper.",
    );
    add("call.read", "tool", "north Juniper port 9999");
    add("memory_verify", "derived", "north Juniper port 9999");
    add("assistant_message", "word", "Archive this report.");
    let q = add(
        "user_message",
        "q1",
        "north 在 Juniper 版是否批准了port 7400？",
    );
    add("user_message", "late", "north Juniper adopted port 8600.");
    let vocab = Vocabulary {
        fields: vec!["port".into()],
        scopes: vec!["north".into()],
        periods: vec!["Juniper".into()],
    };
    let read = || engine.review_context("review", "q1", &vocab).unwrap();
    let ctx = read();
    assert_eq!(ctx.through_event, q - 1);
    assert_eq!(ctx.candidate_ids, vec![first]);
    assert_eq!(ctx.coverage, "named_mentions");
    assert_eq!(
        ctx.originals.iter().map(|r| r.id).collect::<Vec<_>>(),
        [first, denial]
    );
    // An older schema and an empty candidate list still recover the same originals.
    let db = Connection::open(
        work.path()
            .join(".alpha-memory/sessions/review/store.sqlite"),
    )
    .unwrap();
    db.execute_batch("DROP TABLE claim_requests; DROP TABLE arguments; DROP TABLE checks; DROP TABLE claims; DROP TABLE event_fact_tags; PRAGMA user_version=2;")
        .unwrap();
    assert!(read().candidate_ids.is_empty());
    assert_eq!(read().originals.len(), 2);
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn partial_or_ambiguous_review_context_is_explicit() {
    use alpha_memory::Vocabulary;
    let (_ws, _user, engine) = engine();
    let vocab = Vocabulary {
        fields: vec!["port".into(), "budget".into()],
        ..Default::default()
    };
    for _ in 0..17 {
        engine
            .capture("many", "assistant_message", None, "port 1000")
            .unwrap();
    }
    for (key, question, coverage) in [
        ("one", "Which port?", "partial"),
        ("two", "Compare port and budget.", "single_target_required"),
    ] {
        engine
            .capture("many", "user_message", Some(key), question)
            .unwrap();
        let ctx = engine.review_context("many", key, &vocab).unwrap();
        assert_eq!(ctx.coverage, coverage);
        assert!(ctx.originals.len() <= 16);
    }
    assert!(engine
        .review_context("many", "no-such-question", &vocab)
        .is_err());
}
