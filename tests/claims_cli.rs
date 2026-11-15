use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;
fn run(dir: &TempDir, args: &[&str], input: Value, success: bool) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_alpha-memory"))
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--user-home",
            dir.path().join("user").to_str().unwrap(),
        ])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert_eq!(
        result.status.success(),
        success,
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    if success {
        serde_json::from_slice(&result.stdout).unwrap()
    } else {
        Value::Null
    }
}
#[test]
fn independent_process_creation_refutation_and_restoration() {
    let dir = TempDir::new().unwrap();
    run(
        &dir,
        &["append", "--session", "a", "--body", "synthetic original"],
        Value::Null,
        true,
    );
    let input = json!({"key":"finite","definition":"holds for all 16 inputs","scope":"0..16","previous":null,"sources":[1]});
    let c = run(
        &dir,
        &["claim", "add", "--session", "a", "--request", "claim"],
        input.clone(),
        true,
    );
    assert_eq!(
        c,
        run(
            &dir,
            &["claim", "add", "--session", "a", "--request", "claim"],
            input,
            true
        )
    );
    let version = c["event"].as_i64().unwrap().to_string();
    let check = json!({"target":c["event"],"checker":"finite","checker_version":"1","scope":"0..4","coverage":"partial","method":"sample","outcome":"support","sources":[1]});
    let partial = run(
        &dir,
        &["check", "record", "--session", "a", "--request", "sample"],
        check.clone(),
        true,
    );
    assert_eq!(
        partial,
        run(
            &dir,
            &["check", "record", "--session", "a", "--request", "sample"],
            check.clone(),
            true
        )
    );
    let explain = run(
        &dir,
        &["claim", "explain", "--session", "a", "--version", &version],
        Value::Null,
        true,
    );
    assert_eq!(explain["states"][&version]["status"], "sampled");
    let mut full = check.clone();
    full["scope"] = json!("0..16");
    full["coverage"] = json!("complete");
    run(
        &dir,
        &["check", "record", "--session", "a", "--request", "sample"],
        full.clone(),
        false,
    );
    run(
        &dir,
        &["check", "record", "--session", "a", "--request", "full"],
        full,
        true,
    );
    let h = run(
        &dir,
        &[
            "handoff",
            "export",
            "--session",
            "a",
            "--request",
            "export",
            "--versions",
            &version,
        ],
        Value::Null,
        true,
    );
    let event = h["reference"]["event"].as_i64().unwrap().to_string();
    let mut counter = check;
    counter["outcome"] = json!("refute");
    run(
        &dir,
        &["check", "record", "--session", "a", "--request", "counter"],
        counter,
        true,
    );
    run(
        &dir,
        &[
            "handoff",
            "restore",
            "--session",
            "b",
            "--request",
            "restore",
            "--source-session",
            "a",
            "--event",
            &event,
        ],
        Value::Null,
        false,
    );
    let r = run(
        &dir,
        &[
            "handoff",
            "restore",
            "--session",
            "b",
            "--request",
            "restore",
            "--source-session",
            "a",
            "--event",
            &event,
            "--allow-source",
            "a",
        ],
        Value::Null,
        true,
    );
    assert_eq!(r["exported"]["states"][&version]["status"], "verified");
    assert_eq!(r["current"]["states"][&version]["status"], "refuted");
    let shown = run(
        &dir,
        &["claim", "show", "--session", "a", "--version", &version],
        Value::Null,
        true,
    );
    assert_eq!(shown["sources"][0]["body"], "synthetic original");
}

#[test]
fn cli_independent_arguments_preserve_the_surviving_basis() {
    let dir = TempDir::new().unwrap();
    run(
        &dir,
        &["append", "--session", "s", "--body", "synthetic basis"],
        Value::Null,
        true,
    );
    let mut ids = std::collections::BTreeMap::new();
    for key in ["a", "b", "c", "d"] {
        let result = run(
            &dir,
            &["claim", "add", "--session", "s", "--request", key],
            json!({"key":key,"definition":key,"scope":"finite","previous":null,"sources":[1]}),
            true,
        );
        ids.insert(key, result["event"].as_i64().unwrap());
    }
    for key in ["a", "b"] {
        run(
            &dir,
            &[
                "check",
                "record",
                "--session",
                "s",
                "--request",
                &format!("support-{key}"),
            ],
            json!({"target":ids[key],"checker":"fixture","checker_version":"1","scope":"finite","coverage":"complete","method":"enumeration","outcome":"support","sources":[1]}),
            true,
        );
    }
    for (target, premise, request) in [("c", "a", "ac"), ("c", "b", "bc"), ("d", "c", "cd")] {
        let input = json!({"target":ids[target],"premises":[ids[premise]],"sources":[1]});
        let result = run(
            &dir,
            &["argument", "add", "--session", "s", "--request", request],
            input.clone(),
            true,
        );
        assert_eq!(
            result,
            run(
                &dir,
                &["argument", "add", "--session", "s", "--request", request],
                input,
                true
            )
        );
    }
    for (key, expected) in [("a", "supported"), ("b", "unsupported")] {
        run(
            &dir,
            &[
                "check",
                "record",
                "--session",
                "s",
                "--request",
                &format!("refute-{key}"),
            ],
            json!({"target":ids[key],"checker":"fixture","checker_version":"1","scope":"witness","coverage":"partial","method":"counterexample","outcome":"refute","sources":[1]}),
            true,
        );
        let d = ids["d"].to_string();
        let graph = run(
            &dir,
            &["claim", "explain", "--session", "s", "--version", &d],
            Value::Null,
            true,
        );
        assert_eq!(graph["states"][&d]["status"], expected);
    }
}
