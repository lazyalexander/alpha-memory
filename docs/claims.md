# Versioned claims and evidence

The engine records checker reports; it never executes a checker or invents a
proof from a conversation. Domain scope and argument rules belong to the caller.
Old summaries, configuration supersession, inferred tags and task checkpoints
retain their existing meaning. Schema 4 adds claim tables transactionally to
schema 3; no old log is promoted into a verified claim.

## Executable example

Build `cargo build --offline`. Run the following in an empty workspace, using a
separate user directory. `event` in each response is the persistent version or
record identifier. All write commands require a session-local request ID.

```sh
BIN=/path/to/alpha-memory
mkdir -p /tmp/claim-example
cd /tmp/claim-example
export ALPHA_MEMORY_HOME=/tmp/claim-example/user
$BIN append --session experiment --body 'Definition: predicate P holds on inputs 0 through 15.'
printf '%s' '{"key":"p","definition":"P holds","scope":"integers:0..15","previous":null,"sources":[1]}' |
  $BIN claim add --session experiment --request define-p
# In this empty session the claim version is event 2.
$BIN append --session experiment --body 'Checker v1 tested 0,1,2,3 successfully.'
printf '%s' '{"target":2,"checker":"enumerator","checker_version":"1","scope":"integers:0..3","coverage":"partial","method":"enumeration","outcome":"support","sources":[3]}' |
  $BIN check record --session experiment --request sample-p
$BIN claim explain --session experiment --version 2
# Status is sampled, not verified.
$BIN append --session experiment --body 'Checker v1 tested all 16 inputs successfully.'
printf '%s' '{"target":2,"checker":"enumerator","checker_version":"1","scope":"integers:0..15","coverage":"complete","method":"enumeration","outcome":"support","sources":[5]}' |
  $BIN check record --session experiment --request full-p
$BIN claim show --session experiment --version 2
$BIN source --session experiment --event 5
$BIN handoff export --session experiment --request export-p --versions 2
# Export reference is event 7 in this example.
$BIN handoff restore --session continuation --request restore-p \
  --source-session experiment --event 7 --allow-source experiment
```

`claim add`, `check record`, and `argument add` consume typed JSON from stdin.
The corresponding Rust methods take `ClaimInput`, `CheckInput`, and
`ArgumentInput`. A revision repeats the claim key and specifies `previous`.
Only one successor per version can commit. A second root with the same key is
rejected. Definitions and scope strings cannot be updated in place.

An argument is `{"target":8,"premises":[2,6],"sources":[7]}`: every premise
is necessary. Separate argument records are alternatives. The caller supplies
the original reasoning in the source event; the engine tracks whether its
necessary premises still have support. Cycles are rejected atomically.

## Status contract

| Status | Meaning |
| --- | --- |
| `verified` | At least one support check explicitly reports complete coverage with an exactly equal scope string. |
| `supported` | At least one argument has only verified or supported premises. |
| `sampled` | Partial support exists, but no complete check or valid argument. |
| `unsupported` | No currently sufficient basis. |
| `refuted` | A checker reports a counterexample against this exact version. |

Refutation takes priority but does not delete conflicting support. Unknown
coverage, timeouts, unknown outcomes and mismatched complete scopes remain
visible as inconclusive checks. A counterexample may cover one input: the
caller is responsible for establishing that it refutes the target definition.
Scope inclusion and checker credibility are not inferred. Definitions and
metadata containing secret-shaped text are rejected; originals should first be
captured through the existing redacting event API.

`claim explain` returns the dependency graph, all original source bodies,
per-version states, every argument's necessary premise states, and record IDs
for support, partial support, counterexamples and inconclusive checks. Sampled
premises do not establish their dependents. A failed argument does not invalidate
another independent argument. State is recomputed from immutable records on
query; there is no destructive invalidation cache.

## Handoff and authority

Version IDs are session-local. A handoff reference is the pair
`{source_session,event}` and is valid only in the canonical workspace that
created it. One export contains selected versions, their necessary dependencies,
predecessor versions, original source bodies and the source event position.
Multiple source sessions use separately authorized handoff references; this
version does not create cross-session argument edges or copy claims into the
target session. Restore writes a receipt retaining source-qualified references.
Hosts must route later reads and revisions to their owning session.

Restore obtains a SQLite writer reservation on the source, validates original
records, recalculates the graph, then commits one destination receipt and its
idempotency result in a single transaction. Concurrent source writes either
precede that snapshot or wait until the receipt commits. Lock contention returns
`retryable source snapshot unavailable`; it does not claim to observe a later
state. The receipt includes export and checked positions, both explanations,
and changed versions. Permissions are passed separately by the trusted host via
`allowed_sources` / `--allow-source`; a manifest never grants permission.

Retries with the same request and payload return the original result, including
an old restore receipt. Use a new request ID for a fresh recheck. Reusing a
request with different data or operation fails. Authorization is rechecked
before returning a cached restore. Missing sources, changed content, foreign
workspaces and write failures produce no partial receipt. This integrity check
protects references to stored records; it is not authentication against an
attacker who can rewrite the entire SQLite database and export together.

## Validation and performance

`cargo test --offline` covers API and separate-process CLI use, schema-3 upgrade,
rollback injection, conflicting revisions, independent arguments, DAG states,
source integrity and handoff authorization. The legacy test fixtures explicitly
remove schema-4 tables when simulating schema 0–2. `bench/fixtures/v3.sql` is a
synthetic portable baseline fixture, with an FTS rebuild instead of SQLite's
nonportable writable-schema virtual-table dump.

See [acceptance report](../bench/ACCEPTANCE.md) for measured results and pending
external integration work. Raw runs and databases belong in ignored
`bench/runs/`; only synthetic fixtures and sanitized summaries are publishable.
