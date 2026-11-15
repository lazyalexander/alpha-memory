# Step 1 maturity and acceptance

**English** · [简体中文](step1-maturity.zh-CN.md)

**Current maturity: a research prototype with repeatable acceptance. Step 1's
engineering gate passes, allowing controlled ablations to begin. The original
research task is not a full pass; memory effectiveness and production readiness
remain unestablished.**

This delivery hardens the existing native runtime path with failure handling,
independent replay and a repeatable acceptance command. It does not relabel the
previous single-seed 9/10 smoke as system stability.

## Development outcomes

Native CLI calls have a 30-second deadline. Nonzero exits, missing executables,
timeouts, invalid JSON and invalid response envelopes become fatal backend
errors, leaving diagnostics and stopping the current phase without a Python
fallback. Receipts include elapsed time and error type. A tool is acknowledged
only after its required native calls finish.

The auditor now replays every native command with the pinned executable in an
empty database and compares Rust's actual responses, rather than only comparing
matching JSON logs. Replay accepts only known operations/flags, not workspace
paths supplied by a receipt. Regression tests detect an observation and its
matching status receipt being forged consistently.

The no-model acceptance command runs revision and stale-export scenarios from
empty workspaces. Each scenario uses three independent processes, explicit
handoff references and source permissions. It disables the Python support
calculator during actions and uses the independent oracle only afterward.
Failure tests exercise the phase loop, prohibit another inference after a fatal
error and reopen the databases. A temporary slow executable also verifies the
real subprocess deadline, beyond injecting TimeoutExpired.

## Acceptance evidence

| Area | Result | Limit |
| --- | --- | --- |
| Rust core | Previous 39 tests and Clippy passed; core source unchanged this delivery | Reuses the `bff5663` core baseline; no new core-test claim |
| Research harness | 68 tests pass | Includes legacy controls, permissions, runtime oracle exclusion, faults and tamper checks |
| Deterministic process acceptance | 2 scenarios × 3 rounds, all 6 pass | 18 worker processes; stable old version IDs, expected states, sources and empty-DB replay |
| Qwen seeds 43 and 44 | Both complete 3/3 phases at 9/10 | Each trace/native/binary/sequence/replay audit passes; failed actions retained |
| Automatic crash recovery | Incomplete | Compound CLI work is not a distributed transaction; diagnostics may survive and execution stops |
| OpenCode 2.x integration | Incomplete | The existing 1.19 adapter is not 2.x acceptance |
| Paper effectiveness | Unverified | Ablations, unseen tasks and cross-model comparisons remain unrun |

An early acceptance-script draft cited an unread original after restoring an old
handoff and was correctly rejected. The fixture now explicitly reads the source
and retains worker logs. That failed development directory remains local and is
not counted as a later successful acceptance run.

Run in the research repository:

```sh
python3 bench/native_acceptance.py run \
  --binary /path/to/alpha-memory \
  --rounds 3 --output bench/runs/step1-acceptance
```

The [synthetic acceptance summary](../bench/step1-deterministic-summary.json)
contains binary/source hashes and per-scenario outcomes. These engineering
fixtures are not model scores.

## Model behavior and cost

Qwen3-30B-A3B-Instruct-2507 Q8_0 uses the fixed artifact, non-thinking mode,
temperature 0.6, top-p 0.95, a 1,024-token response cap, feedback v2 and the
10/12/14 action budgets. The original ten-condition scorer is unchanged.
Seeds 43/44 are development regressions, not held-out task evaluations.

| Seed | Completed phases | Score | Calls | Tool errors | Model-call time | Runner elapsed |
| --- | --- | --- | --- | --- | --- | --- |
| 42, prior native smoke | 3/3 | 9/10 | 30 | 3 | 135.41 s | Not measured |
| 43, this delivery | 3/3 | 9/10 | 26 | 2 | 101.38 s | 115.11 s |
| 44, this delivery | 3/3 | 9/10 | 31 | 6 | 187.33 s | 202.03 s |

Runner elapsed includes that run's preflight, model hashing and host work,
excluding server startup and post-run auditing. Native CLI time totals about
0.53/0.73 seconds for 64/66 calls; this excludes other host work. Both new runs
have zero source-permission refusals. Model tool errors and backend faults are
counted separately.

All three seeds lose the independent-counterexample condition: the model calls
the exhaustive checker before using the exposed witness. Runner exit code 1 is
therefore the original scorer's failure status, not a crashed phase; phase-failure
fields are null. The original ten conditions have not all passed.

Seed 42 used an earlier native-host version. The finite-checking algorithm is
unchanged here, but timeout plumbing changed its module digest. Sources and
checker identities are recorded separately. These observations are not a
single-variable ablation or a statistical success rate. Host-computed states are
visible to the model, so this is not a measure of unaided reasoning.

The [model regression details](../bench/step1-model-regression.json) retain all
ten scores, phase errors, reads, budgets, costs and audits. Raw conversations,
receipts and databases stay in ignored directories. The temporary server was
stopped.

Historical auditing also covers checker-module-only drift: old evidence uses
frozen source modules, and corrupt archives are rejected. Seeds 42, 43 and 44
were re-audited with the final reader.

## Maturity boundary and next action

The host can now support controlled experiments with repeatable engineering
acceptance and explicit failure signals. It still has one explicitly writable
ledger per study, no general multi-ledger merge or cross-database atomic commit,
no automatic interrupted-write recovery, and no long-running concurrency or
cross-host-version checker-compatibility acceptance. Research code is now pushed to the private
[alpha-memory-research/dev](https://github.com/lazyalexander/alpha-memory-research/tree/dev).
The core [dev branch](https://github.com/lazyalexander/alpha-memory/tree/dev)
contains the plan, core interface and sanitized reports.

Proceed to the four-way ablation while freezing feedback, permissions and scoring,
then preregister unseen tasks/seeds and include full costs. Until those studies
finish, the supported claim is limited to step-1 engineering integration and
repeatable acceptance.
