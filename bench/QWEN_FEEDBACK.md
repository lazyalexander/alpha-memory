# Qwen tool-format feedback treatment — 2026-11-16

**The complete feedback treatment completed all three sessions and scored 9/10.**
The original seed-42 failure remains preserved at 0/10. This is one development
comparison using a known failure, not a held-out evaluation or success rate.

## Change

The research host now returns actionable `tool-feedback/v2` errors:

- Exact field mismatches identify unexpected, missing and allowed fields.
- Invalid JSON, invalid top-level fields and wrong field types consume one action
  and return feedback, allowing the model to retry within its existing budget.
- Citation errors list only sources already visible in the current session and
  explain how to read an additional original before citing it.

Raw outputs and submitted commands are preserved. The host does not strip fields,
synthesize a valid request, repair a model judgment, accept unseen citations or
increase the budget. Truncation and transport errors still terminate a phase.
The trace auditor validates rejected response envelopes against their recorded
feedback. Source-permission metrics match the explicit permission error rather
than treating the word “allowed” in field diagnostics as an authorization denial.

This belongs to the caller-owned research harness; Rust claim semantics are
unchanged. Research regressions pass: 52 tests, including the former expected
failure for citation feedback. Tests cover atomic rejection, explicit repair,
raw-response retention, source visibility, and budget exhaustion.

## Preserved trials

| Treatment | Original score | Completed sessions | Calls | Model-call time |
| --- | ---: | ---: | ---: | ---: |
| Original feedback | 0/10 | 0/3 | 10 | 24.51 s |
| Argument/citation diagnostics only (v1) | 2/10 | 1/3 | 15 | 35.99 s |
| Complete response/argument/citation feedback (v2) | 9/10 | 3/3 | 31 | 128.60 s |

The intermediate v1 trial fixed the phase-A handoff but stopped on an extra
field in the phase-B response envelope. It is retained as a failed development
trial under ignored `bench/runs/qwen-feedback-selftest/`. It motivated the
explicit envelope-error treatment; its score was not changed afterward.

For the final trial, model weights and SHA-256, pinned engine executable, seed,
sampling, response cap, task prompts, assignments, permissions, original scorer
and 10/12/14-action budgets match the original Qwen run. Only feedback and format
error recovery changed. Preregistration and source snapshots remain local.

## Final observations

| Phase | Actions / budget | Tool errors | Active reads | Reads after refusal | Actions remaining |
| --- | ---: | ---: | ---: | ---: | ---: |
| Preliminary | 10 / 10 | 2 | 1 | 0 | 0 |
| Review | 9 / 12 | 2 | 4 | 0 | 3 |
| Revision | 12 / 14 | 2 | 2 | 2 | 2 |

The preliminary researcher repeated the original unexpected `verified_state`
argument at step 9. After receiving the new diagnosis, it removed that field
itself at step 10 and handed off successfully. The reviewer later repaired both
an extra-field response envelope and an invalid citation, reading additional
sources before completing handoff. All three phases completed without truncation.

The remaining failed condition is independent counterexample construction before
checker disclosure: the reviewer enumerated the domain first. The later correct
refutation judgment cannot receive credit for independently proposing that
witness. The original scorer remains unchanged and the overall run is therefore
not an all-conditions pass.

Trace audit, host handoff checks and the post-run native Rust/CLI projection audit
all passed. The final old/new version states agree with evidence; old refutation
and unrelated verification survive both handoffs. The native audit is post-run,
not evidence that the model directly chose native CLI commands.

Final cost: 31 calls, 128,704 prompt tokens, 2,168 completion tokens, 130,872 total;
128.60 seconds of summed model-call time. The original failed trial did less work,
so these durations do not establish an end-to-end speedup.

Sanitized metrics: `qwen-feedback-summary.json`. All raw traces, rejected outputs,
server logs and databases stay in ignored `bench/runs/`. Seeds 43/44 and the
held-out dual-argument task remain unrun. The temporary Qwen server was stopped.
