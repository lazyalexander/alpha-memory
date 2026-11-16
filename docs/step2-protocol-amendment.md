# Step 2 common schema amendment

**English** · [简体中文](step2-protocol-amendment.zh-CN.md)

Recorded before the second pilot, after observing handoff schema failures in the
initial development batch. The [original protocol](step2-protocol.md) and its
archived source remain unchanged. Both batches must appear in the report.

The initial common prompt said “assessment judges EACH selected claim ID” without
stating its JSON type. Multiple cells repeatedly submitted arrays, which the
existing handoff validator correctly refused. This prevents continuation and
confounds a comparison of cross-session memory. This is a protocol defect found
by a pilot, not evidence that any memory arm is ineffective.

The second batch changes this common sentence for **all four arms**:

```text
assessment MUST be a JSON object mapping EACH selected claim ID to a status
string, for example {"key@1":"tested"}; never an array.
```

Arm profiles, feedback v2, checker, validator,
scorer, action/token budgets, model artifact, sampling and run order stay fixed.
The preregistration identifies `explicit-assessment-map/v2`; both sets of source
hashes and exact inference contexts are retained. No initial cell is replaced.
The same development seeds 43/44 are rerun in the same eight-cell order after
the first batch finishes. There is no claim of a held-out or independent repeat.
Results from these prompt revisions must be reported separately, never pooled
as sixteen exchangeable samples. Any repaired workflow remains a development
pilot requiring a later unseen-task evaluation.

## Interpretation note added during report review

The example uses the actual `key@1` ID and `tested`, which matches one phase-A
judgment. The original pre-inference copy did not recognize this overlap; it
remains in local evidence. This is a shared task-specific scaffold as well as a
JSON-type clarification. Do not attribute a between-batch change purely to format.
All four arms in the amended batch receive the same example. Frozen inference
prompts are not changed again. Future unseen-task protocols should use an
example ID independent of the scored fixture.
