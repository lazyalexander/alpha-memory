# alpha-memory

A Rust memory engine for session logs, task checkpoints, evidence retrieval and recall. Callers pass a workspace and session ID through the CLI. Host plugins and model-driven experiments live in separate repositories.

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

### Explicit session recall

The existing `recall` command builds a broader session pack on request.
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

A task with an indexed constraint can be resumed as follows (fresh session):

```sh
alpha-memory append --session demo --config-key staging.probe.port --config-value 4761 --body "Legacy client requirement"
alpha-memory task --session demo --task probe --goal "Maintain staging probes" --pin 1 --hint "Check legacy client compatibility"
alpha-memory context --session demo
alpha-memory session-config --session demo --key staging.probe.port
alpha-memory index --session demo --query "legacy client"
alpha-memory source --session demo --event 1
```

`reflect pending` records only new, contiguous event spans from the same task.
Promoted and superseded ranges remain covered.

Build with `cargo build --locked` and run the engine tests with `cargo test --locked`.
