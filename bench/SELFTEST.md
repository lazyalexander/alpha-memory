# Local model self-test — 2026-11-16

**The seed-42 self-test passed all 10 original conditions and completed all three
sessions. Trace auditing and the native Rust/CLI projection audit also passed.**
This is one controlled local run, not a general model or system success rate.

Model: DeepSeek-R1-Distill-Qwen-32B, Q4_K_M. The existing local artifact's SHA-256
was verified and recorded in `selftest-summary.json`. The candidate executable
was copied before the run so concurrent builds could not replace it.

Sampling: temperature 0.6, top-p 0.95, 1,024 output tokens per call, seed 42 plus
action index. Runtime: local llama-server, Metal GPU, one slot, 16,384-token
context, 384-token reasoning budget. Each phase had its own process and session;
only explicit handoffs carried prior evidence.

| Phase | Completed | Actions / budget | Tool errors | Active reads | Reads after revision refusal |
| --- | --- | --- | --- | --- | --- |
| Preliminary | Yes | 10 / 10 | 2 | 1 | 0 |
| Review | Yes | 12 / 12 | 8 | 1 | 0 |
| Revision | Yes | 14 / 14 | 5 | 0 | 3 |

There were **15 rejected tool operations**, including six invalid-citation
handoff attempts, two attempts to act before restoring, and three revision
attempts without required predecessor reads. The reviewer eventually repaired
its citations on its last action. The final researcher performed three reads
only after admission refusals. All three phases finished with zero remaining
actions. Thus 10/10 does not establish efficient or robust autonomous operation.

Cost: 36 model calls; 133,764 prompt tokens and 15,097 completion tokens
(148,861 total); 1,415.73 seconds of summed call wall time (23.60 minutes).
Qwen was downloading concurrently, so this is not an isolated hardware speed
benchmark. No remote API was used.

The native CLI audit re-read authorized originals and replayed their contents
into session-local claim projections. It observed sampled initial support,
direct refutation of the old key, loss of dependent support, persistence of the
independent claim, and supported/verified revised claims without reviving old
ones. Native handoff export/restore also succeeded.

The historical research harness exposes evidence-derived states to the model.
The native audit occurred after model execution. These results therefore do not
show unaided model proof discovery or model-selected native CLI commands.
No outputs, citations, action budgets or failed attempts were manually repaired.

Machine-readable sanitized results: `selftest-summary.json`.
Raw traces, source snapshots, audit results, database files and logs remain under
ignored `bench/runs/deepseek-selftest/`. Seeds 43/44, the held-out dual-argument
model task, and the separately labeled Qwen condition remain unrun.
