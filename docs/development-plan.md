# Evidence-memory validation development plan

**English** · [简体中文](development-plan.zh-CN.md)

Step 1 is implemented; deterministic checks and a real Qwen run are recorded in [step 1 acceptance](native-runtime-step1.md). Later steps remain planned.

## Research question and ownership

Test whether version-bound evidence and explicit argument dependencies reduce
incorrect reuse of conclusions across sessions, while retaining independent
support. This is a mechanism experiment inspired by a research verification
workflow, not a reproduction of the source paper's mathematical theorem.

alpha-memory owns immutable versions, evidence linkage, argument evaluation and
source-qualified handoff integrity. The research host owns domain checkers,
label presentation, task permissions and optional predecessor-read admission.
OpenCode 2.x is the planned application host after controlled validation. Codex's
extraction/consolidation and source-citation design is a reference and a possible
later comparison. Neither product's native memory becomes the core's authority.

## Delivery sequence

| Step | Deliverable | Acceptance |
| --- | --- | --- |
| 1. Native runtime path | The small research host writes claims, arguments and checks through Rust CLI; reads/explanations and export/restore also use Rust during inference. | Three separate model processes; stable version references; new counterevidence visible; independent evidence preserved; authorization rejected; no runtime fallback to Python-computed states; original scorer retained. |
| 2. Controlled ablations | Summary only; summary + original retrieval; retrieval + versions/evidence/dependencies; full mechanism + predecessor-read admission. | Same model artifact, tool-format feedback, task data and action/token policy across groups. Label host enforcement separately from model initiative. |
| 3. Held-out evaluation | Unseen scopes, dual independent arguments, old exports with newer counterexamples, unrelated claims, distractors and tasks requiring multiple sessions. | Freeze fixtures, scoring and seeds before inference; retain every failed run; do not reuse development seed 42 as held-out evidence. |
| 4. OpenCode 2.x integration | Adapt event capture, tool routing, compaction and session permissions to a pinned 2.x commit. | Actual app tests of capture/compaction/restart/parent-child boundaries. The existing 1.19 adapter is not assumed compatible. |
| 5. External validity | Another model and, later, another host such as Codex behind the same core contracts. | Report model/host changes explicitly and preserve per-condition costs; do not attribute a bundled change to memory alone. |

## Step 1 contract

Keep the legacy host as a separate control; introduce an explicitly selected
native backend. Model actions retain their domain vocabulary. Domain rules may
validate scope and checker results, but runtime support/refutation decisions
come from Rust explanations. The independent Python evaluator is used only after
a model phase for scoring and comparison.

A workspace contains an explicitly host-selected study ledger. Claim versions
remain in that ledger across fresh conversation sessions. Selecting the native
ledger grants the host adapter read/write access; models cannot choose the ledger
or another session through tool arguments. Continuations must first restore a
specific native handoff using host-supplied permissions. Research originals keep
their own session-qualified IDs and content digests. They must be reread with the
separate original-session allowlist; a manifest grants no permission.

The host translates Rust's generic `supported` label into the fixture's
rule-supported `verified`, and uses explanation reasons to distinguish blocked
from invalidated. It exposes the original native labels and source references as
well. This presentation layer may not promote partial coverage or compute an
alternative support graph. Native backend failures stop the phase without a
Python fallback. Individual CLI writes are transactional; a sequence of CLI
calls is not a distributed transaction. Any acknowledged tool success requires
all its native writes to finish; failed partial operations remain diagnostic
records and are never presented as completed work.

Record the backend, ledger policy, binary/source hashes, native call receipts,
model artifact hash, runtime settings, prompts, budgets and scorer identity.
Native records must exist before the next inference request, not be synthesized
only by a post-run projection.

## Measurements and interpretation

Primary: wrong reuse of old conclusions, incorrect invalidation of independent
support, exact evidence/version binding, citation accuracy and workflow
completion. Also report proactive reads, reads after refusal, permission denials,
format errors, calls, all model tokens (including memory maintenance), wall time,
remaining action budget and storage cost.

Freeze `tool-feedback/v2` for the ablation comparison. Changing feedback is a
separate intervention. Use 43/44 as planned regression runs; preregister new seeds
and unseen tasks for held-out evaluation. A single seed or a 9/10 development
score does not establish general memory effectiveness. Never compare latency
from an incomplete phase to a completed three-session task as a speedup.

## Evidence and publication

Synthetic fixtures, tests, runner code and sanitized summaries may be committed.
Raw conversations, database files, local paths, model weights and server logs
remain in ignored local run directories. Core and research results are reported
separately. Each delivery records completed checks, failures and remaining work.

Step 1 now has repeatable process acceptance and 43/44 model regressions; see the [maturity report](step1-maturity.md). It remains a research prototype.
