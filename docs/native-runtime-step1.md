# Step 1 acceptance: live Rust evidence path

The live Rust backend is implemented and its seed-42 Qwen smoke completed all
three fresh model processes. The original scorer returned 9/10. This establishes
an executable integration path, not the effectiveness of memory versus ablations.

## Delivered

- `claims_explain(session, versions)` and `claim explain --version 2,6` read
  multiple roots in one SQLite transaction. The existing single-root Rust method
  and CLI form retain their behavior.
- The research runner has an explicit `--backend native --native-ledger NAME`
  option. The legacy backend remains the default control.
- Native runtime actions write actual claims, arguments and checks before tool
  success is returned. Reads return native explanations. Handoffs use native
  export and restore. Each observation is tied to a native explanation receipt.
- A host-selected, workspace-local study ledger preserves exact version IDs
  across fresh conversations. Its explicit read/write capability is separate
  from the allowlist for original research sessions. The model cannot choose it.
- Python keeps domain checker execution, admission policy and display labels.
  Rust determines support/refutation. The Python support evaluator runs only
  after a phase as an independent oracle. Tests disable it throughout actions.
- Native CLI errors stop the phase. There is no legacy fallback. Compound host
  actions span several transactions; failed partial records remain diagnostics,
  not acknowledged results. Automated recovery of interrupted compound writes
  is outside this delivery.

Implementation lives in the sibling research repository's
`bench/native_runtime.py`, `bench/native_audit.py`, and `bench/research_model.py`.
Run instructions and failure boundaries are in `bench/NATIVE_RUNTIME.md` there.
The core remains model- and host-neutral.

## Verification

39 Rust tests and Clippy passed. The 60-test research suite passed against the
candidate executable in an isolated test directory. Native tests cover three
host instances, revision and scope policy, stable IDs, newer counterevidence on
old exports, independent conclusions, explicit permissions, native failures,
records existing before tool success, fabricated observations and missing
receipts. The actual model run provides the separate-process check.

The Qwen artifact, seed, sampling, task prompts, assignments, feedback v2 and
10/12/14 action budgets match the earlier Qwen feedback condition. The engine
binary and native observation/backend path are deliberately changed and labeled.
The original scorer's AST is unchanged. The result is a development smoke;
seed 42 was already used for debugging.

| Phase | Completed | Actions / budget | Tool errors | Active reads | Reads after refusal | Native CLI calls |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Preliminary | Yes | 10 / 10 | 0 | 3 | 0 | 28 |
| Review | Yes | 10 / 12 | 3 | 4 | 0 | 15 |
| Revision | Yes | 10 / 14 | 0 | 4 | 0 | 26 |

Cost: 30 model calls; 138,869 prompt tokens, 2,007 completion tokens, 140,876 total;
135.41 seconds of summed model-call time. This duration excludes CLI and startup
work and is not an end-to-end latency measurement.

All trace-audit checks passed. Native checks independently confirmed:

1. Observed states bind to the actual step's native CLI response.
2. Every accepted observation matches the independent domain oracle.
3. Claim versions, check records, arguments and original-source bindings exist
   durably, including the earlier snapshots referenced in the run.
4. Native export and restore events exist and bind their exact source references.

The old key remains refuted after revision; dependent old claims remain
unsupported, the independent claim remains verified, and repaired claims are
verified/supported on their own versions. Generic Rust `supported` is shown as
rule-supported `verified` in the fixture vocabulary, with the native label also
visible. The remaining failed score condition is independent counterexample
construction: the reviewer used the exhaustive checker first.

Machine-readable sanitized results: [qwen-native-summary.json](../bench/qwen-native-summary.json).
Full preregistration, pinned binary, source snapshots, model traces, native call
receipts and databases remain under ignored `bench/runs/qwen-native-runtime/`.
The temporary model server was stopped. No raw traces or model files are published.

## Remaining work

The four-way ablation, new held-out tasks/seeds, dual-argument model evaluation,
OpenCode 2.x adapter and Codex comparison are later steps. This backend uses one
explicitly writable ledger per study; it does not add general multi-session
argument edges or distributed transactions. A 9/10 smoke cannot substitute for
those experiments. See the [English plan](development-plan.md) or
[中文计划](development-plan.zh-CN.md).

Subsequent failure hardening, repeatable three-process acceptance and seeds
43/44 are documented in the [maturity report](step1-maturity.md) /
[中文报告](step1-maturity.zh-CN.md). The original smoke above remains unchanged.
