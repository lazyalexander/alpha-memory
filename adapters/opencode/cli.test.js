import assert from "node:assert/strict"
import { mkdtempSync } from "node:fs"
import { tmpdir } from "node:os"
import path from "node:path"
import test from "node:test"
import { fileURLToPath } from "node:url"

import { createClient, formatRecall, textFromParts, uncoveredSpan } from "./cli.js"

const bin = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../target/debug/alpha-memory",
)

test("an OpenCode session id round-trips through recall", () => {
  const workspace = mkdtempSync(path.join(tmpdir(), "alpha-memory-"))
  const userHome = mkdtempSync(path.join(tmpdir(), "alpha-memory-user-"))
  const memory = createClient({ bin, workspace, userHome })
  const session = "ses_018f6e4c9b21AbcdEF0123456789"

  const appended = memory.append(session, ["--body", "Use the session log."])
  assert.equal(appended.ok, true, appended.error)

  const standing = memory.standing("Do not drop data.")
  assert.equal(standing.ok, true, standing.error)

  const recalled = memory.recall(session, false)
  assert.equal(recalled.ok, true, recalled.error)
  assert.equal(recalled.data.long_term_opened, false)
  assert.equal(recalled.data.standing[0].text, "Do not drop data.")
  const text = formatRecall(recalled.data)
  assert.match(text, /Do not drop data/)
  assert.doesNotMatch(text, /## Long-term/)
})

test("text parts skip synthetic notes", () => {
  assert.equal(
    textFromParts([
      { type: "text", text: "hello" },
      { type: "text", text: "hidden", synthetic: true },
      { type: "tool", text: "no" },
    ]),
    "hello",
  )
})

test("uncovered span starts after the last reflected event", () => {
  assert.deepEqual(
    uncoveredSpan(
      [{ id: 1 }, { id: 2 }, { id: 3 }],
      [{ start_event_id: 1, end_event_id: 2 }],
    ),
    { from: 3, to: 3 },
  )
  assert.equal(uncoveredSpan([{ id: 1 }], [{ start_event_id: 1, end_event_id: 1 }]), null)
})
