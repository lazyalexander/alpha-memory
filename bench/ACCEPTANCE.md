# Claim engine acceptance — 2026-11-16

The Rust/CLI implementation and deterministic checks are complete. A subsequent seed-42 local DeepSeek self-test completed all three sessions
and scored 10/10, with 15 tool errors and all action budgets exhausted. See
[the self-test report](SELFTEST.md). The full multi-seed model acceptance is pending.

## Implementation and checks

Baseline: `ba5a19e`; its 26 existing Rust tests passed before implementation.
The baseline CLI help, append/recall outputs and a synthetic schema-3 SQL dump
are preserved in `fixtures/`. The candidate has 38 passing Rust tests:
8 library tests, 18 legacy integration tests, 10 claim integration tests and
two separate-process CLI scenarios. `cargo clippy --offline --all-targets -- -D
warnings` passes. The documented no-model CLI example was executed successfully.

| Plan area | Evidence |
| --- | --- |
| Immutable versions, exact scope, original evidence | `tests/claims.rs`: partial 4-input support, mismatched complete scope, unknown coverage, timeout, full 16-input support, revision and reopen |
| Independent arguments and failure propagation | A and B independently support C; D requires C; first refutation preserves C/D, second removes their support; direct C refutation retains conflicting support; unrelated U survives |
| Independent graph oracle | Fixed-seed 32-node DAG, test-owned iterative expected statuses compared with recursive engine evaluation |
| Transactions and retries | Duplicate requests, changed-payload rejection, concurrent successor race, injected migration/revision/restore failures |
| Handoff | Exported/current states, permission rejection, missing/modified originals, wrong workspace, source writer contention, target receipt rollback |
| User entry point | Creation, checks, export, counterexample and restore run in independent CLI processes |

The schema-0/1/2 legacy tests now remove schema-4 tables when constructing their
old-database fixtures; their original behavioral assertions are retained.

## Performance

Hardware: Apple M5 Max, 64 GiB unified memory, macOS arm64. Both builds use the
same Rust toolchain and bundled SQLite/WAL configuration. See
`performance-metadata.json` for binary/source hashes and build metadata.

Each scale (1,000 / 10,000 / 100,000 initial events) has 100 warmup operations
and five measured rounds of 100 operations. Samples include opening the engine's
SQLite connection; CLI timings also include process launch and JSON output.
Synthetic event contents are constant. Seed 42 identifies the fixed fixture.

- `performance-summary.json`: baseline/candidate legacy API and CLI timings;
  candidate library explain, counterexample-plus-recompute and restore timings.
- `performance-recheck-summary.json`: repeated 1k/10k measurements.
- `recall-recheck-summary.json`: focused repeat of the recheck's 1k CLI recall flag.
- `claims-cli-summary.json`: new-operation CLI p50, p95, throughput and database size.

The first run flagged library source p95 at 1k (+29.7%) and 10k (+47.3%). Neither
flag reproduced in the rerun. The rerun separately flagged 1k CLI recall
(+24.3%); a focused same-fixture repeat measured baseline 5.775 ms and candidate
5.815 ms (+0.69%). No >20% regression reproduced in its corresponding recheck.
These are local engineering measurements, not absolute latency guarantees.

New CLI p95, milliseconds:

| Initial events | Explain | Counterexample + explain (two processes) | Restore |
| ---: | ---: | ---: | ---: |
| 1,000 | 9.46 | 17.06 | 13.68 |
| 10,000 | 9.59 | 16.58 | 13.01 |
| 100,000 | 8.65 | 16.75 | 13.87 |

The new-operation graph is a two-node dependency chain, not a 100k-node graph.
Counterexample measurements append distinct reports, so evidence accumulates.
Restore receipts retain full audit results, and the stress workload grows the
combined databases to approximately 420–439 MB. Large-graph scaling and receipt
storage optimization are not demonstrated by this benchmark. Raw samples and
SQLite files remain in ignored `bench/runs/`.

## Research integration and remaining work

The sibling `alpha-memory-research` repository contains `core_claims.py`,
`core_matrix.py`, `test_core_claims.py`, and a separately preregistered
`dual_argument_model.py` task. The adapter re-reads authorized originals and
checks a session-local projection through the formal CLI. Its three-process
pilot passes. The research Python suite ran against the candidate binary:
42 tests, with one existing expected failure.

The fixed-seed matrix retains the historical model protocol and ten-condition
scorer, then audits records through native claims. It reports phase completion,
active/refusal-triggered reads, errors, usage, time and remaining actions.
It is a post-run core audit, not evidence that a model chose the new CLI tools.
The held-out dual-argument treatment uses native tool observations and is
reported separately; it has not been used for debugging or run with a model.

The existing DeepSeek model was found locally, but its server was not running.
The user subsequently selected Qwen3-30B-A3B-Instruct-2507 Q8_0 for download.
Qwen results must be a separately labeled model condition, not silently passed
through the DeepSeek historical control. Model identity, non-thinking template,
output budget and sampling policy must be frozen before the first scored run.
A subsequent local DeepSeek seed-42 run is documented in `SELFTEST.md`.
Its trace and core-projection audits passed. The two additional DeepSeek seeds and held-out task remain pending.
A subsequent Qwen seed-42 run failed at its first handoff because of an extra
request field; see [the Qwen self-test report](QWEN_SELFTEST.md).

## Version-one boundaries

Claim IDs and argument edges are session-local. A handoff stores source-qualified
references and restores an atomic receipt; it does not copy authoritative claims
into the destination. Multiple source sessions use separately authorized exports.
There is no atomic multi-source bundle operation in this version. Hosts route
later reads/revisions to their owning session. These boundaries are documented
in `docs/claims.md` and must be considered before declaring broader host
integration complete. No commit, push or release was performed.

A subsequent feedback treatment completed all three Qwen sessions and scored
9/10, with trace and native audits passing. See [the feedback report](QWEN_FEEDBACK.md).
The research feedback regression suite now has 52 passing tests. This is a
single-seed development comparison; multi-seed acceptance remains pending.

The next delivery connects live inference to the Rust backend, rather than a
post-run projection. It passes 39 Rust tests and 60 research tests and completed
a three-process Qwen smoke at 9/10 with live receipts audited. See
[step 1 acceptance](../docs/native-runtime-step1.md); ablations remain pending.

Step 1 hardening now passes 66 research tests, six repeated deterministic
process scenarios and audited Qwen 43/44 regressions (both 9/10, three phases
complete). See [maturity](../docs/step1-maturity.md) for evidence and limits.

The latest step-1 hardening passes 68 research tests, six deterministic
three-process scenarios and audited 43/44 regressions. See
[step-1 maturity](../docs/step1-maturity.md) / [中文成熟度](../docs/step1-maturity.zh-CN.md).
Historical scores above remain separate from this delivery.

Step 2 adds the A/B/C/D controlled pipeline and raises the research suite to
83 passing tests. Two separate eight-cell development batches are retained and
audited. The first has a common schema failure; the amended batch completes 5/8
workflows, with 3/8 fully correct assessments and 6/8 final evidence states.
See [the bilingual step-2 report](../docs/step2-report.md) /
[中文](../docs/step2-report.zh-CN.md). These are known-task pilots; held-out
validation and OpenCode 2.x integration remain pending.
