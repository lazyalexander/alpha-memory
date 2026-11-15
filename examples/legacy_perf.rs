//! Portable benchmark: also builds unchanged against ba5a19e.
use alpha_memory::{Engine, RecallQuery};
use std::time::Instant;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let workspace = std::path::Path::new(&args[1]);
    let n: usize = args[2].parse().unwrap();
    std::fs::create_dir_all(workspace).unwrap();
    let e = Engine::open(workspace, workspace.join("user")).unwrap();
    e.capture("perf", "log", None, "synthetic event").unwrap();
    let mut c =
        rusqlite::Connection::open(workspace.join(".alpha-memory/sessions/perf/store.sqlite"))
            .unwrap();
    let tx = c.transaction().unwrap();
    {
        let mut s = tx
            .prepare("INSERT INTO events(body,redacted,created_at) VALUES ('synthetic event',0,0)")
            .unwrap();
        for _ in 1..n {
            s.execute([]).unwrap();
        }
    }
    tx.commit().unwrap();
    let mut results = serde_json::Map::new();
    for op in ["write", "source", "recall"] {
        let mut rounds = vec![];
        for round in 0..6 {
            let mut samples = vec![];
            for _ in 0..100 {
                let t = Instant::now();
                match op {
                    "write" => {
                        std::hint::black_box(
                            e.capture("perf", "log", None, "synthetic event").unwrap(),
                        );
                    }
                    "source" => {
                        std::hint::black_box(e.source("perf", 1, 0).unwrap());
                    }
                    _ => {
                        std::hint::black_box(
                            e.recall(RecallQuery {
                                session_id: "perf".into(),
                                task_id: None,
                                open_long_term: false,
                                stage: None,
                                recent: 5,
                            })
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
    c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    println!(
        "{}",
        serde_json::json!({"samples_ms":results,"events":n,"db_bytes":std::fs::metadata(workspace.join(".alpha-memory/sessions/perf/store.sqlite")).unwrap().len()})
    );
}
