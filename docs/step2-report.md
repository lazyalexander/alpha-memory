# Step 2 development pilot report

**English** · [简体中文](step2-report.zh-CN.md)

**Maturity: an audited controlled-ablation research prototype.** The comparison
pipeline is implemented and tested. The amended pilot completes 5/8
three-session workflows; 3/8 have all 15 judgments correct, and 6/8
reach the required final evidence state. These are two development seeds on one
known task, not evidence of general superiority or production readiness.

## Comparison and controls

See the [original protocol](step2-protocol.md) and the pre-inference
[common-schema amendment](step2-protocol-amendment.md). A receives a previous
model-authored summary and claim handles; B adds original retrieval; C adds
computed evidence states, version/provenance bindings and explanations; D adds
mandatory predecessor-original reads. A can read its current-session originals.
All groups share the private native storage, checker, scope rules and permissions.
The causal scope is public memory information and admission policy; C bundles
multiple information components, and storage-engine effects are not isolated.

The model is Qwen3-30B-A3B-Instruct-2507 Q8_0 on Apple M5 Max / 64 GiB,
served with the bundled llama-server, non-thinking mode, 16,384 context tokens,
one parallel slot, 99 GPU layers, eight threads, batch 512 / microbatch 256,
cache RAM zero and no context shift. Temperature 0.6, top-p 0.95, response cap
1,024, `cache_prompt:false`, feedback v2 and action budgets 10/12/14 stay fixed.
Exact model/binary/source and preregistration hashes are in the public summaries.

Preregistration here is a local pre-inference hash freeze, not an external registry.
Each batch freezes source snapshots and exact prompts before inference and uses
A43/B43/C43/D43/D44/C44/B44/A44 order. Processes and message histories are fresh;
only explicit handoffs carry work. Summary generation occurs in ordinary model
handoffs and incurs normal tokens. No extra summarizer or post-phase oracle
supplies a continuation. Seeds 43/44 and the task were already used in development.
The two prompt revisions are reported separately, never pooled as sixteen trials.

## Initial batch: retained protocol failure

All eight cells exhausted phase A's budget without a handoff. They made 80 model
requests and had 22 rejected assessment shapes. The common prompt had
omitted that assessment is a JSON object keyed by claim ID; the model repeatedly
submitted arrays. Every cell's projection/context/native-replay audit passes,
but none reaches a cross-session comparison. Every cell has 15 missing required
judgments; zero accepted false promotions is not a success when no judgments exist.

| Arm/seed | Sessions | Correct judgments | Missing | False verified | Final evidence | Requests | Returned tokens | Model s | Runner s | Audit |
| --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |
| A43 | 0/3 | 0/15 | 15 | 0 | no | 10 | 17423 | 15.7 | 16.4 | pass |
| B43 | 0/3 | 0/15 | 15 | 0 | no | 10 | 19207 | 18.7 | 19.0 | pass |
| C43 | 0/3 | 0/15 | 15 | 0 | no | 10 | 29242 | 28.1 | 28.4 | pass |
| D43 | 0/3 | 0/15 | 15 | 0 | no | 10 | 29299 | 51.2 | 51.5 | pass |
| D44 | 0/3 | 0/15 | 15 | 0 | no | 10 | 28998 | 42.8 | 43.4 | pass |
| C44 | 0/3 | 0/15 | 15 | 0 | no | 10 | 29186 | 36.8 | 37.4 | pass |
| B44 | 0/3 | 0/15 | 15 | 0 | no | 10 | 19248 | 25.4 | 25.9 | pass |
| A44 | 0/3 | 0/15 | 15 | 0 | no | 10 | 17391 | 20.4 | 20.9 | pass |

The amendment clarifies the assessment JSON type for all arms. Its shared
`key@1:tested` example also matches one phase-A judgment: this is a task-specific
scaffold, so the between-batch change is not a pure format intervention. Validators, task answers, feedback, scoring, budgets, profiles and model
stay unchanged. This repair was motivated by observed pilot failures and is not
a held-out intervention. Original cells and archived code remain intact.

## Amended batch: every cell

| Arm/seed | Sessions | Correct judgments | Missing | False verified | Final evidence | Requests | Returned tokens | Model s | Runner s | Audit |
| --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |
| A43 | 3/3 | 10/15 | 0 | 2 | yes | 27 | 59195 | 67.8 | 69.7 | pass |
| B43 | 3/3 | 11/15 | 0 | 0 | yes | 31 | 77656 | 87.7 | 89.6 | pass |
| C43 | 3/3 | 15/15 | 0 | 0 | yes | 28 | 127459 | 143.3 | 145.1 | pass |
| D43 | 3/3 | 15/15 | 0 | 0 | yes | 32 | 153321 | 178.3 | 180.3 | pass |
| D44 | 2/3 | 8/15 | 7 | 0 | no | 33 | 150014 | 163.7 | 165.4 | pass |
| C44 | 3/3 | 15/15 | 0 | 0 | yes | 27 | 115859 | 121.2 | 122.6 | pass |
| B44 | 2/3 | 7/15 | 7 | 0 | yes | 31 | 79139 | 81.7 | 83.1 | pass |
| A44 | 2/3 | 6/15 | 7 | 0 | no | 30 | 73312 | 80.4 | 81.9 | pass |

A43 incorrectly promotes two blocked derived claims to verified in phase A;
A43 and B43 later understate fully verified repaired claims as tested. D44
completes only two sessions: in phase C, format and read-gate refusals are
followed by missing resubmissions and checks of nonexistent claim IDs. Nine
tool errors consume the 14-action budget; its final handoff is refused and
seven judgments remain missing. This is a model workflow failure with a passing
integrity audit, not a backend crash. There is no retry beyond the frozen cap.
B44 reaches the final evidence state but loses its last handoff: an invalid
citation is followed by reads through the final action, leaving no action for
handoff. Seven judgments remain missing even though the scientific work is done.
This separates evidence construction from a completed cross-session workflow.
A44 samples `key@2`, leaving it tested and its dependents blocked, then spends
budget on additional @3/@4 strategies. Its last handoff has an invalid citation;
seven final judgments are missing. Later versions do not repair the required @2.
A read gate therefore has an observed completion risk under this policy;
these two seeds do not identify a general C-versus-D effect. Some failures
concern tool/citation planning and cannot be attributed solely to memory content.

The table counts accepted handoff judgments against independently replayed evidence.
Incorrect and missing judgments are distinct. Across this batch, false invalidation
of the independent finding occurs 0 times; fatal backend errors 0,
inference transport/truncation failures 0 (not task-reasoning failures). Per-phase reads, refusals, remaining actions, individual
legacy scores, all token components and storage/CLI costs remain in the
[allowlisted amended summary](../bench/step2-ablation-revised.public.json).
The [initial summary](../bench/step2-ablation-initial.public.json) is retained too.
The original ten-condition score includes reads unavailable to A, so it is
reported descriptively and cannot fairly rank these arms.

Returned token usage totals are server-reported prompt plus completion tokens,
including repeated context and handoff text; 0 requests across both batches
lack complete returned usage. Model time includes all attempted request wall time.
Runner time excludes shared server startup, artifact hashing and post-run audit;
native CLI time covers the tracked native-ledger calls, not every source-event
operation. Neither an incomplete workflow nor a shorter context is evidence of
a completed-task speedup. No significance test or general effect estimate is made.

## Acceptance and report material

Research tests: **83 pass**, including complete three-phase pipelines for all
four arms, wrong-summary preservation, A/B state exclusion, D-only admission,
cross-arm rejection, oracle exclusion, failed/truncated requests, public/private
tampering, empty-database replay and publication privacy/unknown-usage checks.
Core Rust source is unchanged; the prior 39-test/Clippy baseline is reused.
Each batch has eight reported cells and all eight pass exact-context, projection,
scorer identity, durable-record and native CLI replay audits.

Raw materials stay in ignored local runs: frozen protocol copies, preregistration
and digest, source snapshots, request attempts, raw responses, public actions,
private native actions, CLI receipts, phase results, model-authored summaries,
handoffs, databases, worker/server logs and execution metadata. A local evidence
index hashes the final artifacts. Public summaries contain only aggregate metrics
and build/protocol hashes; they do not independently reproduce the raw observations.
See the [materials manifest](../bench/step2-materials.public.json) for test and
inventory hashes. The private index is `bench/runs/step2-evidence-index.json`.
Code and run instructions: [research implementation](https://github.com/lazyalexander/alpha-memory-research/tree/a852d04/bench).

## Next gate

Freeze separate unseen tasks, scopes, scoring and new seeds before any further
model inference. The reserved dual-argument task has not been used for this pilot.
Include two independent justifications, stale exports with newer counterevidence,
unrelated claims and distractors. Report workflow completion and incorrect reuse
together, then estimate variability on more task/seed pairs. Keep OpenCode 2.x and
cross-model integration after that validation. Step 2's engineering/pilot gate is
complete; formal effectiveness, unseen-task generalization and production
recovery remain unverified.
