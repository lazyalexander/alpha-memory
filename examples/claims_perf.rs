use alpha_memory::*;
use std::time::Instant;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let workspace = std::path::Path::new(&args[1]);
    let e = Engine::open(workspace, workspace.join("user")).unwrap();
    let a = e
        .claim_add(
            "perf",
            "root",
            ClaimInput {
                key: "root".into(),
                definition: "finite claim".into(),
                scope: "0..16".into(),
                previous: None,
                sources: vec![1],
            },
        )
        .unwrap()
        .event;
    let b = e
        .claim_add(
            "perf",
            "child",
            ClaimInput {
                key: "child".into(),
                definition: "dependent".into(),
                scope: "0..16".into(),
                previous: None,
                sources: vec![1],
            },
        )
        .unwrap()
        .event;
    e.argument_add(
        "perf",
        "arg",
        ArgumentInput {
            target: b,
            premises: vec![a],
            sources: vec![1],
        },
    )
    .unwrap();
    let h = e.handoff_export("perf", "export", &[b]).unwrap();
    let mut results = serde_json::Map::new();
    for op in ["explain", "propagation", "restore"] {
        let mut rounds = vec![];
        for round in 0..6 {
            let mut samples = vec![];
            for i in 0..100 {
                let t = Instant::now();
                match op {
                    "explain" => {
                        std::hint::black_box(e.claim_explain("perf", b).unwrap());
                    }
                    "propagation" => {
                        e.check_record(
                            "perf",
                            &format!("check-{round}-{i}"),
                            CheckInput {
                                target: a,
                                checker: "finite".into(),
                                checker_version: "1".into(),
                                scope: Some("7".into()),
                                coverage: Coverage::Partial,
                                method: "counterexample".into(),
                                outcome: CheckOutcome::Refute,
                                sources: vec![1],
                            },
                        )
                        .unwrap();
                        std::hint::black_box(e.claim_explain("perf", b).unwrap());
                    }
                    _ => {
                        std::hint::black_box(
                            e.handoff_restore(
                                "dest",
                                &format!("restore-{round}-{i}"),
                                h.reference.clone(),
                                &["perf".into()],
                            )
                            .unwrap(),
                        );
                    }
                }
                samples.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            if round > 0 {
                rounds.push(samples);
            }
        }
        results.insert(op.into(), serde_json::json!(rounds));
    }
    println!("{}", serde_json::json!({"samples_ms":results}));
}
