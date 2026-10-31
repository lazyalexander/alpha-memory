# alpha-memory

alpha-memory is a long-term memory system for coding-agent harnesses. Several agents and a person share one memory. Memory is a first-class part of the system: it is governed, not pasted back into the prompt as a transcript. The aim is to stay useful across long, hard problems.

Language models compress the time a person needs to do the work. They do not replace the person. Collaboration is the point.

Governed memory is there to:

1. Lower the chance of hallucinations and harmful accidents.
2. Support the rest of the system: noticing when the environment changes, auditing token spend, and checking safety.

## Record and recall

Writing and reading are separate paths. One score does not do both jobs.

### Write

Every agent interaction is appended to a write-ahead log. The log is the source of truth. A payload is redacted before it becomes durable.

The same write updates the config slot table. A slot is one key, its latest value, and the log position that value came from. The event writes the slot. Reflection does not.

### Read

The mid-term context of the current task is present by default.

A small standing set is also present. Intent recognition cannot drop it. The set holds the safety floor and user preferences that do not change with the task.

Intent recognition runs on the hot path, when the user speaks. It looks at the user's need and the current task. It does not read long-term bodies. It answers one question: should past long-term memory be opened?

If it is opened, two filters apply:

- **config** matches by exact key. A key that already appears in the last few steps is left out of the injection. The log entry stays.
- **env** matches loosely, on user profile and development stage. A record carries the stage it was written under, and recall prefers the current stage.

Reflection stays off this path. Its token cost is metered on its own.

## Kinds

`eventKind` places each record. Four kinds are required.

| Kind | What it holds | How recall treats it |
| --- | --- | --- |
| `config` | Exact values: port, branch, test command, model name | Slot table. The latest value wins. Recent steps suppress injection. |
| `env` | User profile, development stage, and other slow context | Loose match on profile and stage. |
| `decision` | A choice that should hold | Recalled as a decision. A later choice marks it superseded. |
| `failure` | An attempt that failed | Kept as its own record, so a later session can find it. |

Config and env do not cover the log. A failed attempt is not a config value. A hard constraint is not a profile. Either one, folded into an environment summary, is gone for the next session.

## Reflection

Reflection reads a span of the log and writes mid-term entries. It runs at session end, before context compaction, and when the task changes.

Long-term reflection reads mid-term entries only.

A long-term entry points at the mid-term entries it came from. A mid-term entry points at a log range. When a config key or a decision is replaced, the old entry is marked superseded. Both versions are not rewritten into one longer paragraph.

Narrative text may say that a config key changed. The value lives in the slot, with its log position. Exact artifact identity, a path and a content hash, stays structured the same way.

## Where it runs

The engine is one Rust program: the log, the slot table, reflection, and recall. Codex, OpenCode, and DeepSeek Harness (`dsh`) share that store. The same hooks-and-MCP shape fits other harnesses.

Harness differences stay in thin adapters.

- Codex calls the program from lifecycle hooks and an MCP server.
- OpenCode and `dsh` load a small TypeScript plugin. The plugin registers tools and injects context. Storage and recall stay in the Rust program.

The shared Rust store is the source of truth for every harness, including `dsh`.

## Status

This repository is the design and the start of the engine. The behavior above is the contract the implementation has to meet.
