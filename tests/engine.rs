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
