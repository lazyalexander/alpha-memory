import path from "node:path"
import { fileURLToPath } from "node:url"
import { tool } from "@opencode-ai/plugin"

import {
  createClient,
  formatRecall,
  resolveBinary,
  textFromParts,
  truncate,
  uncoveredSpan,
} from "../../adapters/opencode/cli.js"

const here = path.dirname(fileURLToPath(import.meta.url))
const localBin = path.resolve(here, "../../target/debug/alpha-memory")

// OpenCode 1.19 loads every function export from `.opencode/plugins/*.js`.
export async function AlphaMemory(ctx) {
  const memory = createClient({
    bin: resolveBinary(process.env.ALPHA_MEMORY_BIN, localBin),
    workspace: ctx.directory,
  })

  return {
    "experimental.chat.system.transform": async (input, output) => {
      if (!input.sessionID) return
      const pack = memory.recall(input.sessionID, false)
      if (!pack.ok) {
        output.system.push(`## alpha-memory\n${pack.error}`)
        return
      }
      const text = formatRecall(pack.data)
      if (text) output.system.push(text)
    },
    "chat.message": async (input, output) => {
      const text = textFromParts(output.parts)
      if (!text) return
      memory.append(input.sessionID, ["--body", text, "--event-kind", "user_message"])
    },
    "tool.execute.after": async (input, output) => {
      if (input.tool.startsWith("memory_")) return
      memory.append(input.sessionID, [
        "--body",
        truncate(`${input.tool}\n${output.output ?? ""}`),
        "--event-kind",
        input.tool,
      ])
    },
    "experimental.session.compacting": async (input, output) => {
      const recorded = reflectUncovered(memory, input.sessionID)
      if (!recorded.ok) return
      output.context.push(
        recorded.id
          ? `alpha-memory stored mid-term ${recorded.id} for this session. Call memory_keep with the decision that should outlast compaction.`
          : "alpha-memory has no new session events to reflect.",
      )
    },
    tool: {
      memory_recall: tool({
        description:
          "Read this session's alpha-memory. Set open_long_term only when past long-term memory is needed.",
        args: {
          open_long_term: tool.schema.boolean().optional().describe("Open past long-term memory"),
        },
        async execute(args, context) {
          const pack = memory.recall(context.sessionID, Boolean(args.open_long_term))
          if (!pack.ok) return pack.error
          return formatRecall(pack.data) || "No memory for this session."
        },
      }),
      memory_note: tool({
        description:
          "Record a decision, failure, config value, stage, behavioral preference, or standing constraint.",
        args: {
          kind: tool.schema
            .enum(["decision", "failure", "config", "env", "preference", "standing"])
            .describe("Where the note belongs"),
          text: tool.schema.string().describe("The note"),
          topic: tool.schema.string().optional().describe("Stable id for a decision or config key"),
          key: tool.schema.string().optional().describe("Config key"),
          value: tool.schema.string().optional().describe("Config value"),
          stage: tool.schema.string().optional().describe("Development stage"),
        },
        async execute(args, context) {
          if (args.kind === "preference") {
            const saved = memory.preference(args.text)
            return saved.ok ? `Saved preference ${saved.data.id}.` : saved.error
          }
          if (args.kind === "standing") {
            const saved = memory.standing(args.text)
            return saved.ok ? `Saved standing constraint ${saved.data.id}.` : saved.error
          }
          const flags = ["--body", args.text]
          if (args.topic) flags.push("--topic", args.topic)
          if (args.kind === "decision") flags.push("--decision")
          if (args.kind === "failure") flags.push("--failure")
          if (args.kind === "config") flags.push("--config-key", args.key ?? "", "--config-value", args.value ?? "")
          if (args.kind === "env") flags.push("--stage", args.stage ?? "")
          const saved = memory.append(context.sessionID, flags)
          if (!saved.ok) return saved.error
          return `Recorded ${saved.data.kind ?? "log"} event ${saved.data.event_id}.`
        },
      }),
      memory_keep: tool({
        description:
          "Promote this session's mid-term entries into long-term memory. The text is the summary you write.",
        args: {
          text: tool.schema.string().describe("Long-term summary"),
          topic: tool.schema.string().optional().describe("Topic this summary replaces"),
        },
        async execute(args, context) {
          const reflected = reflectUncovered(memory, context.sessionID)
          if (!reflected.ok) return reflected.error
          const pack = memory.recall(context.sessionID, false)
          if (!pack.ok) return pack.error
          const midIds = pack.data.mid_term.map((entry) => entry.id)
          if (midIds.length === 0) return "Nothing to keep."
          const kept = memory.reflectLong(context.sessionID, midIds, args.text, args.topic)
          if (!kept.ok) return kept.error
          return `Kept long-term ${kept.data.id} from mid-term ${midIds.join(",")}.`
        },
      }),
    },
  }
}

function reflectUncovered(memory, session) {
  const events = memory.log(session)
  if (!events.ok) return events
  const pack = memory.recall(session, false)
  if (!pack.ok) return pack
  const span = uncoveredSpan(events.data, pack.data.mid_term)
  if (!span) return { ok: true, id: null }
  const mid = memory.reflectMid(session, span.from, span.to)
  if (!mid.ok) return mid
  return { ok: true, id: mid.data.id }
}
