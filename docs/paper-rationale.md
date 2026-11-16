# From paper verification methods to alpha-memory

**English** · [简体中文](paper-rationale.zh-CN.md)

Reviewed 2026-11-16. Implementation/evidence baseline: core `a8df593`, research `a852d04`.

**We adopt inspectable conditions and evidence for conclusions, and turn that method into a hypothesis about cross-session memory.** The database, version graph and read gate are our engineering choices. The paper does not establish their effectiveness for language models.

## Paper and interpretation

The repository cites *A Proof of the Most Informative Boolean Function Conjecture*, pinned here to [arXiv:2609.24931v2](https://arxiv.org/abs/2609.24931v2). For uniform binary input X, independently flipped output Y and Boolean g, the authors claim a proof of

$$I(g(X);Y)\leq 1-H_2(p),$$

attained by a single-coordinate function. Their route combines a local Bellman inequality, analysis and certified computation. Numerical validation covers specified regions, separating coverage from arithmetic; a research counterexample defeats a stronger generalization, not its principal target conjecture. See theorem 1.1, section 18 and Part 4 in the [paper](https://arxiv.org/html/2609.24931v2).

The authors publish a [Lean formalization and verification instructions](https://github.com/dpwoodru/general-courtade-kumar-lean). We have not independently rebuilt that proof. The earlier README could conflate an open research stage with the paper's final proof claim; it is corrected with this report. [Stellar Colosseum](https://arxiv.org/abs/2609.15983v2) is a separate harness paper, not a system reproduced here.

## Why adopt the methods

A handoff saying “checks passed” can omit its domain and coverage. A subsequent session may promote four successful samples into a universal claim. Changing an implementation can also make an old test irrelevant. Conversely, rejecting one justification need not defeat a separate sufficient justification.

These are concrete memory failure modes. We therefore choose to preserve definitions, scope, versions, checker receipts and explicit dependencies. A new session should be able to inspect why a conclusion holds and what changed. This is our transfer from verification practice to memory design; it is not a consequence of the mutual-information bound. That bound does not select a summary length, embedding method or storage capacity.

The approach is useful because it separates enforceable contracts from behavioral hypotheses. Version binding and partial-coverage rules can be tested deterministically. Reduced incorrect reuse by a model needs controlled empirical evidence.

## Implementation

| Design choice | Existing implementation | Limit |
| --- | --- | --- |
| Bind evidence to a precise claim and domain | `ClaimInput` stores definition, scope, predecessor and sources; `CheckInput` stores target, checker/version, coverage and outcome | Scope equality is exact; containment is not inferred |
| Preserve partial and inconclusive checks | `check record` distinguishes complete/partial/unknown; timeouts remain visible | The host must supply a trustworthy checker report |
| Preserve revision history | `claim add` creates immutable successors; checks retain their original targets | New success cannot repair old records or redirect edges |
| Retain alternative justifications | Premises within one argument are jointly necessary; separate arguments are alternatives; `claim explain` recomputes states/reasons | The host owns domain rules and the validity of the inference itself |
| Make handoffs inspectable | Export/restore validates sources, checks authorization and compares exported/current states | A manifest grants no permission; hashes do not authenticate against whole-database rewriting |
| Separate model opinions from evidence | Rust supplies runtime states; the independent Python oracle runs after a phase; model judgments remain unchanged | A correct state supplied by the host does not guarantee a correct model handoff |
| Optionally require predecessor reads | D enforces the read gate; C supplies the same information without it | This is our intervention, not a paper requirement; refusals consume actions |

See [contracts](claims.md), [core implementation](../src/claims.rs), [core tests](../tests/claims.rs), [separate-process CLI tests](../tests/claims_cli.rs) and the pinned [research host](https://github.com/lazyalexander/alpha-memory-research/tree/a852d04/bench).

Core `verified` means a matching complete-support report was recorded; `supported` means an argument has sufficient premises. The research host presents domain-rule support as verified and distinguishes blocked from invalidated. These labels are not a new mathematical proof. Direct counterevidence against the target takes priority while conflicting support remains recorded. Version IDs are session-local; the research host explicitly selects a shared ledger and separately authorizes original sources. Restore does not create arbitrary cross-session argument edges. Individual CLI writes are transactional; compound host operations stop on failure without an automatic distributed rollback or recovery guarantee.

## Concrete task

For pairs in {0,1,2,3} squared, a sum key initially passes four diagonal samples. It is only tested; dependent cache/release claims remain blocked. Product commutativity is independently checked over all 16 inputs.

The pairs (0,3) and (1,2) share sum 3 but have products 0 and 2. A validated collision refutes the old key and removes its dependent support while preserving commutativity. A new tuple-key version can receive a complete finite check and support new dependent claims. Old failures remain attached to old versions. Export/restore lets another process inspect those references and subsequent counterevidence.

This finite task says nothing about exhaustive verification on an infinite domain. Alternative independent arguments already have deterministic core tests; their unseen model task remains pending.

## Expected effects and falsification

These are proposed expectations for later evaluation, not achieved guarantees or an already frozen step-3 preregistration.

| Expectation | Measurement | Evidence against the expectation |
| --- | --- | --- |
| Less promotion of samples or stale conclusions to verified | False verified judgments, wrong-version reuse and scope errors under matched model/task/budgets | Fewer answers or more interrupted workflows masquerading as fewer errors |
| Preserve independent support after a refutation | Sequentially defeat two alternatives and inspect downstream/unrelated claims | Over-invalidation after the first defeat, or surviving support after the last |
| Improve inspection and continuation | Source reads, correct judgments, final evidence and all handoffs | Explanations are available but ignored, or completed evidence has no valid handoff |
| Buy reliability at a defensible cost | All attempted requests, tokens, model/CLI time and storage | No replicated benefit or substantial loss of completion from larger context/refusals |
| Make required rereads worthwhile | Compare D with equally informed C; separate proactive and refusal-triggered reads | More refusals and budget exhaustion without less incorrect reuse |

Synthetic acceptance aims for zero version, permission and propagation violations in its specified fixtures. It does not promise zero model errors on open tasks. Broader unseen evaluation must establish variability before selecting acceptable completion/cost thresholds.

## Current evidence

The [amended development batch](../bench/step2-ablation-revised.public.json) has only seeds 43/44 on one known task. Each cell requires 15 judgments; missing judgments stay in the denominator.

| Arm | Information/policy | Correct / 30 | Missing | Complete workflows / 2 | Total tokens |
| --- | --- | ---: | ---: | ---: | ---: |
| A | Model summary and handles | 16 | 7 | 1 | 132,507 |
| B | A plus original retrieval | 18 | 7 | 1 | 156,795 |
| C | B plus native states, version/provenance bindings and explanations | 30 | 0 | 2 | 243,318 |
| D | C plus mandatory rereads | 23 | 7 | 1 | 303,335 |

C merits further testing; these observations do not establish general superiority. D44 lost its final handoff after refusals, omitted resubmissions and budget exhaustion. B44 built the final evidence but did not finish its handoff. A44 did not completely check the required revision. Enforcement can harm completion.

All arms share private native storage and scientific checks. The comparison does not isolate Rust, immutable storage or an individual C information field. The initial eight-cell schema-failure batch is retained separately. The revised prompt's example matches one task judgment, so the between-batch change is not a pure format intervention. Neither batch is held out, and they are not sixteen exchangeable effectiveness trials. See the [pilot report](step2-report.md) and [amendment](step2-protocol-amendment.md).

Recorded engineering evidence includes 83 passing research tests and all 16 cells passing context/projection/native-replay audits, not 16 successful tasks. Core source is unchanged and reuses its prior 39-test/Clippy baseline. Raw evidence stays local; [public materials](../bench/step2-materials.public.json) contain aggregates and hashes.

## Decision and next validation

Continue adopting inspectable scope, evidence, dependencies and provenance. Treat C as a candidate for further validation; whether to enable D by default still needs completion and cost evidence.

Following the [development plan](development-plan.md), freeze unseen tasks, scoring and new seeds before inference: independent alternatives, changed scopes, stale exports with newer counterevidence and distractors. Compare with the same model and budget before OpenCode 2.x integration. Codex-style summaries/citations can be a later comparison; they do not replace the evidence ledger.

The intended capability answers: which version, which domain, which evidence, and what support remains after a change? Current maturity is a controlled research prototype; unseen-task and cross-model effects remain unverified.
