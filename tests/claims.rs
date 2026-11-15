use alpha_memory::*;
use rusqlite::Connection;
use tempfile::TempDir;

fn setup() -> (TempDir, Engine) {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), dir.path().join("user")).unwrap();
    engine
        .capture(
            "s",
            "definition",
            Some("source"),
            "synthetic original evidence",
        )
        .unwrap();
    (dir, engine)
}
fn add(e: &Engine, key: &str) -> i64 {
    e.claim_add(
        "s",
        key,
        ClaimInput {
            key: key.into(),
            definition: format!("{key} holds"),
            scope: "0..16".into(),
            previous: None,
            sources: vec![1],
        },
    )
    .unwrap()
    .event
}
fn check(
    e: &Engine,
    id: i64,
    r: &str,
    outcome: CheckOutcome,
    coverage: Coverage,
    scope: Option<&str>,
) -> i64 {
    e.check_record(
        "s",
        r,
        CheckInput {
            target: id,
            checker: "enumerator".into(),
            checker_version: "1".into(),
            scope: scope.map(String::from),
            coverage,
            method: "enumeration".into(),
            outcome,
            sources: vec![1],
        },
    )
    .unwrap()
    .event
}
fn support(e: &Engine, id: i64, r: &str) {
    check(
        e,
        id,
        r,
        CheckOutcome::Support,
        Coverage::Complete,
        Some("0..16"),
    );
}
fn refute(e: &Engine, id: i64, r: &str) {
    check(e, id, r, CheckOutcome::Refute, Coverage::Partial, Some("7"));
}
fn argument(e: &Engine, id: i64, premises: Vec<i64>, r: &str) {
    e.argument_add(
        "s",
        r,
        ArgumentInput {
            target: id,
            premises,
            sources: vec![1],
        },
    )
    .unwrap();
}
fn status(e: &Engine, id: i64) -> ClaimStatus {
    e.claim_explain("s", id).unwrap().states[&id].status
}
fn db(dir: &TempDir) -> Connection {
    Connection::open(dir.path().join(".alpha-memory/sessions/s/store.sqlite")).unwrap()
}

#[test]
fn exact_scope_versions_reopen_and_idempotency() {
    let (dir, e) = setup();
    let a = add(&e, "a");
    check(
        &e,
        a,
        "four",
        CheckOutcome::Support,
        Coverage::Partial,
        Some("0..4"),
    );
    assert_eq!(status(&e, a), ClaimStatus::Sampled);
    check(
        &e,
        a,
        "wrong",
        CheckOutcome::Support,
        Coverage::Complete,
        Some("0..32"),
    );
    assert_eq!(status(&e, a), ClaimStatus::Sampled);
    check(
        &e,
        a,
        "unknown",
        CheckOutcome::Support,
        Coverage::Unknown,
        None,
    );
    check(
        &e,
        a,
        "timeout",
        CheckOutcome::Timeout,
        Coverage::Complete,
        Some("0..16"),
    );
    assert_eq!(status(&e, a), ClaimStatus::Sampled);
    support(&e, a, "all");
    assert_eq!(status(&e, a), ClaimStatus::Verified);
    let next = ClaimInput {
        key: "a".into(),
        definition: "revised".into(),
        scope: "0..32".into(),
        previous: Some(a),
        sources: vec![1],
    };
    let b = e.claim_add("s", "rev", next.clone()).unwrap();
    assert_eq!(e.claim_add("s", "rev", next.clone()).unwrap(), b);
    let mut changed = next.clone();
    changed.scope = "0..64".into();
    assert!(e.claim_add("s", "rev", changed).is_err());
    assert!(e.claim_add("s", "fork", next).is_err());
    drop(e);
    let e = Engine::open(dir.path(), dir.path().join("user")).unwrap();
    assert_eq!(status(&e, a), ClaimStatus::Verified);
    assert_eq!(status(&e, b.event), ClaimStatus::Unsupported);
    let v = e.claim_show("s", a).unwrap();
    assert_eq!(v.checks.len(), 5);
    assert_eq!(v.sources[0].body, "synthetic original evidence");
}
#[test]
fn dual_arguments_diamond_counterexample_and_cycle_rollback() {
    let (dir, e) = setup();
    let a = add(&e, "a");
    let b = add(&e, "b");
    let c = add(&e, "c");
    let d = add(&e, "d");
    let unrelated = add(&e, "u");
    support(&e, a, "sa");
    support(&e, b, "sb");
    support(&e, unrelated, "su");
    argument(&e, c, vec![a], "ac");
    argument(&e, c, vec![b], "bc");
    argument(&e, d, vec![c], "cd");
    assert_eq!(status(&e, d), ClaimStatus::Supported);
    let before = e.list_events("s").unwrap().len();
    assert!(e
        .argument_add(
            "s",
            "cycle",
            ArgumentInput {
                target: a,
                premises: vec![d],
                sources: vec![1]
            }
        )
        .is_err());
    assert_eq!(e.list_events("s").unwrap().len(), before);
    refute(&e, a, "ra");
    assert_eq!(status(&e, c), ClaimStatus::Supported);
    assert_eq!(status(&e, d), ClaimStatus::Supported);
    let exp = e.claim_explain("s", c).unwrap();
    assert!(!exp.states[&c].arguments[0].valid);
    assert!(exp.states[&c].arguments[1].valid);
    refute(&e, b, "rb");
    assert_eq!(status(&e, c), ClaimStatus::Unsupported);
    assert_eq!(status(&e, d), ClaimStatus::Unsupported);
    support(&e, c, "sc");
    refute(&e, c, "rc");
    let exp = e.claim_explain("s", c).unwrap();
    assert_eq!(exp.states[&c].status, ClaimStatus::Refuted);
    assert_eq!(exp.states[&c].supporting_checks.len(), 1);
    assert_eq!(status(&e, unrelated), ClaimStatus::Verified);
    drop(e);
    let e = Engine::open(dir.path(), dir.path().join("user")).unwrap();
    assert_eq!(status(&e, d), ClaimStatus::Unsupported);
}
#[test]
fn restore_rechecks_authority_integrity_and_atomic_failure() {
    let (dir, e) = setup();
    let a = add(&e, "a");
    let u = add(&e, "u");
    support(&e, a, "sa");
    support(&e, u, "su");
    let h = e.handoff_export("s", "export", &[a, u]).unwrap();
    refute(&e, a, "ra");
    assert!(e
        .handoff_restore("dest", "restore", h.reference.clone(), &[])
        .is_err());
    assert!(!dir.path().join(".alpha-memory/sessions/dest").exists());
    let restored = e
        .handoff_restore("dest", "restore", h.reference.clone(), &["s".into()])
        .unwrap();
    assert_eq!(restored.exported.states[&a].status, ClaimStatus::Verified);
    assert_eq!(restored.current.states[&a].status, ClaimStatus::Refuted);
    assert_eq!(restored.current.states[&u].status, ClaimStatus::Verified);
    assert_eq!(restored.changes.len(), 1);
    assert_eq!(
        restored,
        e.handoff_restore("dest", "restore", h.reference.clone(), &["s".into()])
            .unwrap()
    );
    db(&dir)
        .execute("UPDATE events SET body='corrupt' WHERE id=1", [])
        .unwrap();
    assert!(e
        .handoff_restore("bad", "restore", h.reference, &["s".into()])
        .unwrap_err()
        .to_string()
        .contains("content mismatch"));
    assert!(!dir.path().join(".alpha-memory/sessions/bad").exists());
}
#[test]
fn migration_from_frozen_v3_and_failure_rolls_back() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join(".alpha-memory/sessions/s");
    std::fs::create_dir_all(&p).unwrap();
    let conn = Connection::open(p.join("store.sqlite")).unwrap();
    conn.execute_batch(include_str!("../bench/fixtures/v3.sql"))
        .unwrap();
    // Force migration failure after the first DDL statement.
    conn.execute_batch("CREATE TABLE checks(blocker INTEGER);")
        .unwrap();
    let e = Engine::open(dir.path(), dir.path().join("user")).unwrap();
    assert!(e.capture("s", "log", None, "new").is_err());
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='claims'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    conn.execute_batch("DROP TABLE checks;").unwrap();
    e.capture("s", "log", None, "new").unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        e.list_events("s").unwrap()[0].body,
        "synthetic baseline definition"
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM claims", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn concurrent_successor_has_one_winner() {
    let (dir, e) = setup();
    let a = add(&e, "a");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let path = dir.path().to_path_buf();
            let b = barrier.clone();
            std::thread::spawn(move || {
                let e = Engine::open(&path, path.join("user")).unwrap();
                b.wait();
                e.claim_add(
                    "s",
                    &format!("r{n}"),
                    ClaimInput {
                        key: "a".into(),
                        definition: "revision".into(),
                        scope: "0..16".into(),
                        previous: Some(a),
                        sources: vec![1],
                    },
                )
                .is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>(),
        1
    );
    assert_eq!(e.list_events("s").unwrap().len(), 3);
}
#[test]
fn fixed_seed_dags_match_independent_iterative_oracle() {
    let (_dir, e) = setup();
    let mut rng = 42u64;
    let mut next = || {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        rng >> 32
    };
    let mut ids = vec![];
    let mut expected = vec![];
    for i in 0..32 {
        let id = add(&e, &format!("n{i}"));
        let direct = next() % 4 == 0;
        let refuted = next() % 7 == 0;
        if direct {
            support(&e, id, &format!("support{i}"));
        }
        if refuted {
            refute(&e, id, &format!("refute{i}"));
        }
        let mut supported = direct;
        if i > 0 {
            for j in 0..2 {
                let p = next() as usize % i;
                let q = next() as usize % i;
                argument(&e, id, vec![ids[p], ids[q]], &format!("arg{i}-{j}"));
                supported |= expected[p] && expected[q];
            }
        }
        expected.push(supported && !refuted);
        ids.push(id);
        let actual = status(&e, id);
        let expected_status = if refuted {
            ClaimStatus::Refuted
        } else if direct {
            ClaimStatus::Verified
        } else if supported {
            ClaimStatus::Supported
        } else {
            ClaimStatus::Unsupported
        };
        assert_eq!(actual, expected_status, "node {i}");
    }
}
#[test]
fn injected_write_failures_preserve_event_positions() {
    let (dir, e) = setup();
    let a = add(&e, "a");
    let before = e.list_events("s").unwrap().len();
    db(&dir).execute_batch("CREATE TRIGGER fail_claim BEFORE INSERT ON claims BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(e
        .claim_add(
            "s",
            "rev",
            ClaimInput {
                key: "a".into(),
                definition: "new".into(),
                scope: "0..16".into(),
                previous: Some(a),
                sources: vec![1]
            }
        )
        .is_err());
    assert_eq!(e.list_events("s").unwrap().len(), before);
    let h = e.handoff_export("s", "export", &[a]).unwrap();
    e.capture("dest", "log", None, "destination").unwrap();
    let c = Connection::open(dir.path().join(".alpha-memory/sessions/dest/store.sqlite")).unwrap();
    c.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON claim_requests BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(e
        .handoff_restore("dest", "r", h.reference, &["s".into()])
        .is_err());
    assert_eq!(e.list_events("dest").unwrap().len(), 1);
}
#[test]
fn concurrent_source_writer_returns_retryable_without_import() {
    let (dir, e) = setup();
    let a = add(&e, "a");
    let h = e.handoff_export("s", "export", &[a]).unwrap();
    let c = db(&dir);
    c.execute_batch("BEGIN IMMEDIATE").unwrap();
    let err = e
        .handoff_restore("dest", "r", h.reference, &["s".into()])
        .unwrap_err();
    assert!(err.to_string().contains("retryable"));
    assert!(!dir.path().join(".alpha-memory/sessions/dest").exists());
}

#[test]
fn handoff_missing_and_wrong_workspace_fail_before_destination_write() {
    let (dir, e) = setup();
    let a = add(&e, "a");
    let h = e.handoff_export("s", "export", &[a]).unwrap();
    let conn = db(&dir);
    conn.execute("DELETE FROM events WHERE id=1", []).unwrap();
    assert!(e
        .handoff_restore("missing", "r", h.reference.clone(), &["s".into()])
        .unwrap_err()
        .to_string()
        .contains("missing source"));
    assert!(!dir.path().join(".alpha-memory/sessions/missing").exists());
    let other = TempDir::new().unwrap();
    let p = other.path().join(".alpha-memory/sessions/s");
    std::fs::create_dir_all(&p).unwrap();
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .unwrap();
    std::fs::copy(
        dir.path().join(".alpha-memory/sessions/s/store.sqlite"),
        p.join("store.sqlite"),
    )
    .unwrap();
    let foreign = Engine::open(other.path(), other.path().join("user")).unwrap();
    assert!(foreign
        .handoff_restore("dest", "r", h.reference, &["s".into()])
        .unwrap_err()
        .to_string()
        .contains("workspace"));
    assert!(!other.path().join(".alpha-memory/sessions/dest").exists());
}

#[test]
fn argument_and_export_retries_are_exact_and_partial_premises_do_not_prove() {
    let (_dir, e) = setup();
    let a = add(&e, "a");
    let b = add(&e, "b");
    check(
        &e,
        a,
        "sample",
        CheckOutcome::Support,
        Coverage::Partial,
        Some("0..4"),
    );
    let input = ArgumentInput {
        target: b,
        premises: vec![a],
        sources: vec![1],
    };
    let first = e.argument_add("s", "arg", input.clone()).unwrap();
    assert_eq!(first, e.argument_add("s", "arg", input).unwrap());
    assert_eq!(status(&e, b), ClaimStatus::Unsupported);
    let h = e.handoff_export("s", "export", &[b]).unwrap();
    support(&e, a, "full");
    assert_eq!(h, e.handoff_export("s", "export", &[b]).unwrap());
    assert!(e.handoff_export("s", "export", &[a]).is_err());
    assert_eq!(status(&e, b), ClaimStatus::Supported);
    let restored = e
        .handoff_restore("dest", "r", h.reference, &["s".into()])
        .unwrap();
    assert_eq!(restored.changes.len(), 2);
}

#[test]
fn batch_explanations_include_independent_roots_and_keep_original_versions() {
    let (_dir, e) = setup();
    let a = add(&e, "a");
    let u = add(&e, "u");
    support(&e, a, "sa");
    support(&e, u, "su");
    let graph = e.claims_explain("s", &[a, u]).unwrap();
    assert_eq!(graph.roots, vec![a, u]);
    assert_eq!(graph.states.len(), 2);
    assert_eq!(graph.states[&a].status, ClaimStatus::Verified);
    assert_eq!(graph.states[&u].status, ClaimStatus::Verified);
    refute(&e, a, "ra");
    let graph = e.claims_explain("s", &[a, u]).unwrap();
    assert_eq!(graph.states[&a].status, ClaimStatus::Refuted);
    assert_eq!(graph.states[&u].status, ClaimStatus::Verified);
    assert!(e.claims_explain("s", &[]).is_err());
}
