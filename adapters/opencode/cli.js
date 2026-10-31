import { spawnSync } from "node:child_process"
import { existsSync } from "node:fs"

const OUTPUT_LIMIT = 8000

export function createClient({ bin, workspace, userHome }) {
  if (!bin) throw new Error("alpha-memory binary is not set")
  return {
    bin,
    recall(session, openLongTerm = false) {
      return call(bin, workspace, userHome, [
        "recall",
        "--session",
        session,
        ...(openLongTerm ? ["--open-long-term"] : []),
      ])
    },
    append(session, args) {
      return call(bin, workspace, userHome, ["append", "--session", session, ...args])
    },
    log(session) {
      return call(bin, workspace, userHome, ["log", "--session", session])
    },
    reflectMid(session, from, to) {
      return call(bin, workspace, userHome, [
        "reflect",
        "mid",
        "--session",
        session,
        "--from",
        String(from),
        "--to",
        String(to),
      ])
    },
    reflectLong(session, midIds, body, topic) {
      const args = [
        "reflect",
        "long",
        "--session",
        session,
        "--mids",
        midIds.join(","),
        "--body",
        body,
      ]
      if (topic) args.push("--topic", topic)
      return call(bin, workspace, userHome, args)
    },
    preference(text) {
      return call(bin, workspace, userHome, ["user", "preference", "--text", text])
    },
    standing(text) {
      return call(bin, workspace, userHome, ["user", "standing", "--text", text])
    },
  }
}

export function resolveBinary(envBin, localBin) {
  if (envBin && existsSync(envBin)) return envBin
  if (localBin && existsSync(localBin)) return localBin
  return "alpha-memory"
}

export function formatRecall(pack) {
  const lines = []
  pushList(lines, "Standing constraints", pack.standing)
  pushList(lines, "Behavioral preferences", pack.preferences)
  if (pack.stage?.stage) {
    lines.push("## Stage", pack.stage.stage, pack.stage.body || "")
  }
  if (pack.config?.length) {
    lines.push("## Config")
    for (const slot of pack.config) lines.push(`${slot.key}=${slot.value}`)
  }
  pushEvents(lines, "Decisions", pack.recent_decisions)
  pushEvents(lines, "Failures", pack.recent_failures)
  if (pack.mid_term?.length) {
    lines.push("## Mid-term")
    for (const entry of pack.mid_term) lines.push(entry.body)
  }
  if (pack.artifacts?.length) {
    lines.push("## Artifacts")
    for (const artifact of pack.artifacts) lines.push(`${artifact.path} ${artifact.hash}`)
  }
  if (pack.long_term_opened && pack.long_term?.length) {
    lines.push("## Long-term")
    for (const entry of pack.long_term) lines.push(entry.body)
  }
  if (pack.long_term_opened && pack.env?.length) {
    lines.push("## Earlier stage notes")
    for (const event of pack.env) lines.push(event.body)
  }
  if (lines.length === 0) return ""
  return ["## alpha-memory", ...lines].filter((line) => line !== "").join("\n")
}

export function textFromParts(parts) {
  return (parts ?? [])
    .filter((part) => part && part.type === "text" && !part.synthetic && !part.ignored)
    .map((part) => part.text ?? "")
    .join("\n")
    .trim()
}

export function truncate(text) {
  if (text.length <= OUTPUT_LIMIT) return text
  return `${text.slice(0, OUTPUT_LIMIT)}\n[truncated]`
}

export function uncoveredSpan(events, mids) {
  const covered = new Set()
  for (const mid of mids ?? []) {
    for (let id = mid.start_event_id; id <= mid.end_event_id; id += 1) covered.add(id)
  }
  const open = (events ?? []).filter((event) => !covered.has(event.id))
  if (open.length === 0) return null
  return { from: open[0].id, to: open[open.length - 1].id }
}

function pushList(lines, title, notes) {
  if (!notes?.length) return
  lines.push(`## ${title}`)
  for (const note of notes) lines.push(`- ${note.text}`)
}

function pushEvents(lines, title, events) {
  if (!events?.length) return
  lines.push(`## ${title}`)
  for (const event of events) lines.push(event.body)
}

function call(bin, workspace, userHome, args) {
  const result = spawnSync(bin, [...args, ...scopeArgs(workspace, userHome)], {
    encoding: "utf8",
  })
  if (result.error) {
    return { ok: false, error: result.error.message }
  }
  if (result.status !== 0) {
    return { ok: false, error: (result.stderr || result.stdout || "alpha-memory failed").trim() }
  }
  try {
    return { ok: true, data: JSON.parse(result.stdout) }
  } catch {
    return { ok: false, error: "alpha-memory returned non-JSON output" }
  }
}

function scopeArgs(workspace, userHome) {
  const args = []
  if (workspace) args.push("--workspace", workspace)
  if (userHome) args.push("--user-home", userHome)
  return args
}
