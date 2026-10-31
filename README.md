# alpha-memory

alpha-memory is a long-term memory system for coding-agent harnesses. Several agents and a person can share a session, and every session shares one user directory. Memory is a first-class part of the system: it is governed, not pasted back into the prompt as a transcript. The aim is to stay useful across long, hard problems.

Language models compress the time a person needs to do the work. They do not replace the person. Collaboration is the point.

Governed memory is there to:

1. Lower the chance of hallucinations and harmful accidents.
2. Support the rest of the system: noticing when the environment changes, auditing token spend, and checking safety.

## Caller and model

The engine does not call a model. Intent and prose belong to the harness that already has one. alpha-memory trusts that caller:

- `recall --open-long-term` means the caller's model decided that past long-term memory is needed. Without the flag, long-term entries stay out.
- A body passed to `reflect` is prose the caller's model wrote. An omitted body stores a structural entry: event range, kind counts, config keys, artifact identities, and mid-term ids.

The engine still enforces its own rules. Payloads are redacted before they become durable. The log is append-only. A long-term entry points at mid-term ids, and a mid-term entry points at a log range. A replaced config key or decision topic marks the old entry superseded instead of folding both versions into one paragraph.

## Record and recall

Writing and reading are separate paths.

### Write

Every agent interaction is appended to the session log. The log is the source of truth for that session.

The same write updates the config slot table when it carries a key and a value. A slot is one key, its latest value, and the log position that value came from. The event writes the slot. Reflection does not. The topic defaults to the config key, so a later change can supersede narrative entries about that key.

Belonging is a pure function of the caller's signals, in this order:

1. `--failure`
2. `--decision`
3. an `--event-kind` of `config`, `env`, `decision`, or `failure`
4. a config key and value
5. a `--stage`

Anything else stays in the log and is not promoted on its own. A failure that also carries a config pair is recorded as a failure, and the slot still updates. One event is not both a failure and a decision.

### Read

Recall always returns:

- standing constraints and behavioral preferences, from the user directory
- active mid-term entries for the session
- decision and failure events that no mid-term span covers yet; for a repeated topic, only the latest decision
- config slots whose keys do not appear in the last few events (five, unless `--recent` says otherwise)
- the current stage event
- artifact path, hash, and size

`--open-long-term` adds active long-term entries, and older events from the current stage. Pass `--stage` to choose that stage instead of the latest one. A key that appears in the recent events is left out of the slot list. The log entry stays. The harness already has those steps in the live context.

## Where a record lives

| Record | Place | Lifetime |
| --- | --- | --- |
| Behavioral preferences | user directory | across sessions, always recalled |
| Standing constraints | user directory | across sessions, always recalled |
| Log, slots, stage, decisions, failures, mid-term, long-term, artifacts | session directory inside the workspace | that session only |

The user directory is `$ALPHA_MEMORY_HOME`, or `~/.alpha-memory` when the variable is unset. Session files are `<workspace>/.alpha-memory/sessions/<session-id>/store.sqlite`. Resume a session by passing the same workspace and the same session id. A lesson that must outlive the session is written by the caller into a preference or a standing constraint.

## Kinds

| Kind | What it holds | How recall treats it |
| --- | --- | --- |
| `config` | Exact values: port, branch, test command, model name | Slot table. The latest value wins. Recent steps suppress injection. |
| `env` | Development stage for this session | The current stage is always visible. Older events of that stage need `--open-long-term`. |
| `decision` | A choice that should hold | Visible until a mid-term span covers it. A later event with the same topic marks it superseded. |
| `failure` | An attempt that failed | Kept as its own record in the session, until a mid-term span covers it. |

## Reflection

The caller runs reflection at session end, before context compaction, and when the task changes.

`reflect mid` reads a span of the session log and writes a mid-term entry. `reflect long` reads mid-term ids only, writes a long-term entry, and marks those mid-term entries promoted. Promoted mid-term entries leave the default recall pack. Their long-term entry is visible only when the caller opens long-term memory.

## Where it runs

The engine is one Rust program: the log, the slot table, reflection, and recall. Codex, OpenCode, and DeepSeek Harness (`dsh`) can share a session when they pass the same workspace and session id. They already share the user directory.

Harness differences stay in thin adapters. Those adapters are not in this version. Codex calls the program from lifecycle hooks and an MCP server. OpenCode and `dsh` load a small TypeScript plugin that registers tools and injects context. Storage and recall stay in the Rust program.

## OpenCode

The installed match is OpenCode 1.19. A plugin at `.opencode/plugins/alpha-memory.js` is loaded automatically in this repository. It shells out to `alpha-memory`. Set `ALPHA_MEMORY_BIN` when the debug binary at `target/debug/alpha-memory` is not the one to use.

OpenCode session ids (`ses_` plus letters and digits) are used as alpha-memory session ids. The project directory is the workspace. The engine still does not decide intent: every turn injects the standing set and the current session, and long-term memory stays closed until the model calls `memory_recall` with `open_long_term`. `memory_keep` stores the summary the model writes. Compaction records a structural mid-term entry over events that are not covered yet.

## Commands

```sh
alpha-memory user standing --text "Do not run destructive commands without confirmation."
alpha-memory user preference --text "Prefer small patches."
alpha-memory append --session s1 --config-key port --config-value 4080
alpha-memory append --session s1 --decision --topic db --body "Use SQLite for the session log."
alpha-memory recall --session s1
alpha-memory recall --session s1 --open-long-term
alpha-memory reflect mid --session s1 --from 1 --to 2 --topic db --body "The session log is SQLite."
alpha-memory reflect long --session s1 --mids 1 --topic db --body "Keep the session log in SQLite."
```

`--body -` reads stdin. `--workspace` selects the repo. `--user-home` selects the user directory for one invocation.

## Status

This version is the Rust engine and the `alpha-memory` command. It implements the contract above. Harness plugins are not included yet.
