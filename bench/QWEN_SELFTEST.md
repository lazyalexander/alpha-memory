# Qwen local self-test — 2026-11-16

**Model loading and tool execution worked, but the seed-42 workflow failed at
the first handoff. No later phase was started.** The unchanged original scorer
returned 0/10 because it requires accepted handoffs and downstream evidence.
That score does not mean all initial claim states were wrong.

## Model and controls

The model resides in the user's Desktop/model directory, under
`Qwen3-30B-A3B-Instruct-2507-Q8_0/`. Artifact:
`qwen3-30b-a3b-instruct-2507-q8_0.gguf`, 32,483,930,432 bytes.
The full file hash was rechecked before this run:
`66cbfc7624628ca4725a6a0581b95f2155aa6aa073a6ed4ff8d158794ee8fee3`.
The fixed source revision is
`ggml-org/Qwen3-30B-A3B-Instruct-2507-Q8_0-GGUF@c44c6e74a5f720e4f933d6e6fba8aa0b8f4e8a1d`.

The local llama-server used Metal GPU, a 16,384-token context, one slot, and
explicit non-thinking mode. The original assignments, ten-condition scorer,
10/12/14-action budgets, temperature 0.6, top-p 0.95, seed 42 plus action index,
and 1,024-token response cap were retained. The candidate engine executable was
pinned before execution. This is a separately labeled Qwen condition; it is not
a DeepSeek historical-control rerun.

## Observed result

| Item | Result |
| --- | --- |
| Model identity and full-file SHA-256 | Passed |
| Accepted claim definitions | Four |
| Accepted checks | Full commutativity check; four diagonal key samples |
| Active original reads | One |
| Tool refusals | Three |
| Accepted handoffs / completed phases | Zero |
| Original ten-condition score | 0/10 |
| Calls / actions used | 10 / 10 |
| Prompt / completion tokens | 29,351 / 768 |
| Summed model-call wall time | 24.51 seconds |
| Trace audit | Passed |
| Native core projection audit | Passed for the records from phase A only |

The refusal sequence is concrete:

1. At action 7, the model attempted an off-diagonal counterexample during the
   restricted preliminary phase. The host rejected it.
2. At action 9, the model attempted handoff with an extra `verified_state` field.
   The tool accepts exactly `claim_ids`, `assessment`, `progress`, `next_step`,
   and `citations`, so it rejected the payload.
3. At action 10, the model repeated the same invalid payload and exhausted its
   budget. No artificial handoff was supplied to seed later phases.

The attempted assessment correctly labeled key@1 as tested, its dependents as
blocked, and commutes@1 as verified. The native Rust projection independently
returned sampled, unsupported, unsupported and verified, respectively. The
projection audit also exercised native export for those records; native restore
across phases was not exercised because phases B/C did not run.

## Interpretation and next step

This run establishes that the downloaded Qwen artifact works locally with the
runtime and can produce valid tool-action JSON. It does not establish a complete
three-session workflow. Compared with the previous DeepSeek run, Qwen used fewer
model-call seconds but stopped after phase A; the runtimes are not comparable
end-to-end task latencies.

A useful next treatment is clearer tool-schema feedback, or native structured
tool calling, followed by a separately labeled run. Preserve this failure and
the original scorer. Do not drop the extra field automatically or increase this
run's budget after observing the failure. Seeds 43/44 and the held-out task
remain unrun.

Machine-readable metrics: `qwen-selftest-summary.json`. Full preregistration,
server properties, responses, tool observations and audits remain in ignored
`bench/runs/qwen-selftest/`. The model file stays outside the repository.

A subsequent, separately labeled feedback treatment is documented in
[QWEN_FEEDBACK.md](QWEN_FEEDBACK.md). The original failure above is retained.
