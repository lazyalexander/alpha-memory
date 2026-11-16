# Step 2 development ablation protocol

**English** · [简体中文](step2-protocol.zh-CN.md)

Status: implementation and 81 tests pass; this protocol is recorded before the
first model pilot. This is a development pilot on the existing finite-pair task,
not held-out evidence for general memory effectiveness.

## Conditions and causal scope

| Arm | Cross-session information visible to the model | Revision admission |
| --- | --- | --- |
| A | Prior model-authored progress, next step and assessment; claim handles | No mandatory predecessor reads; prior original retrieval unavailable |
| B | A plus original definitions/check receipts through explicit read | No mandatory predecessor reads |
| C | B plus native support states, version/provenance bindings and explanations | No mandatory predecessor reads |
| D | Same information as C | Current predecessor-original reads enforced before revision |

All arms use the same private native ledger, immutable record identities, domain
checker, scope/derivation validation and source permissions. This controls storage
and checker implementation: the experiment measures exposed memory information
and read-admission policy, not Rust versus another storage engine or the isolated
effect of storing immutable versions. C bundles computed state and provenance;
its individual components are not separately identified by these four arms.

A can still read originals created in its current conversation and run the same
permitted scientific checks. Baseline summaries are the previous model's own
handoff fields, including possibly wrong judgments. There is no extra summarizer;
summary generation is part of the ordinary model handoff and its tokens count.
Host-computed state stays private in A/B. Handles are not original-body reads.

## Frozen pilot controls

- Qwen3-30B-A3B-Instruct-2507 Q8_0, fixed artifact SHA-256, non-thinking mode.
- Existing finite-pair task and phase assignments; fresh processes and empty
  message histories; only explicit handoffs carry earlier work.
- Common instructions and tool schemas; the memory-profile JSON is the sole
  arm-specific prompt section. Capability differences are intentional treatments.
- Feedback v2, temperature 0.6, top-p 0.95, 1,024 response-token cap,
  `cache_prompt:false`, local server/runtime configuration pinned.
- Action budgets 10/12/14. Rejections, format repair and failed model requests
  consume actions. Truncation/transport failure stops that phase; no extra steps
  or artificial handoff are supplied.
- Seeds 43 and 44, used for development regression already. Order: A43, B43, C43,
  D43, D44, C44, B44, A44. This reverses arm order in the second seed block but
  is not a large randomized study.
- Model and engine identity plus server settings are checked; source code is
  archived and cells execute from that archive. Preregistration is hashed before
  inference. All cells are retained, including failed/incomplete ones.

The original ten-condition scorer remains unchanged and descriptive only. Its
read-use conditions are structurally unavailable to A, so its total cannot rank
these arms fairly.

## Arm-neutral outcomes

Primary records include all 15 required phase/claim judgments, missing claims or
judgments, unsupported promotions to verified, false invalidation of the
independent finding, completion of all three handoffs, and the fixed final task's
actual evidence state. Missing work is reported as missing, never a zero-error
success. Completion and judgment correctness are reported together.

Also report source/citation refusals, history-profile refusals, format errors,
fatal backend errors, model failures, proactive reads, reads after refusal and
remaining action budget. Distinguish host enforcement from model initiative.

Costs include attempted model requests, returned usage, prompts/completions,
model-call wall time, cell runner elapsed, native CLI time and database size.
Requests without returned usage are labeled unknown rather than free. Cell
runner elapsed excludes the shared model hash/server startup and post-run audit.
No summarizer or extra model call is hidden outside the cost accounting.

## Isolation and evidence

Public observations use a positive field allowlist. A/B cannot obtain native
state through resume, read, handoff or allowed-citation feedback. Explicit
history-read refusal in A reveals no original content. Wrong model summaries
are preserved, not corrected. Only D enforces the read gate. A packet cannot be
restored into a different arm.

Each cell retains attempted requests (including transport failures), raw model
responses, actual public actions, private native actions, native CLI receipts,
original events/databases, model summaries, phase outcomes, logs and source
snapshots. The auditor compares the exact inference context with public history,
checks the public/private projection and independently recomputes outcome
metrics. Private native receipts are checked and replayed in an empty database.
No post-phase oracle is returned to the model or used to seed a continuation.

Only synthetic fixtures, code, protocol and allowlisted aggregate metrics may
be pushed. Raw runs, local paths, conversations and model weights stay local.

## Interpretation and next gate

Eight cells on one known task can validate the comparison pipeline and expose
failure modes. They cannot establish statistical superiority or generalization.
Do not optimize prompts against these results and label them held-out later.
After auditing this pilot, freeze a separate task/seed set for the planned
unseen-task evaluation; the unused dual-argument task remains reserved.
