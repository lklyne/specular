/**
 * Pure mapping from Claude Agent SDK stream messages to the short,
 * human-readable FixProgressEvents the Comments panel log renders.
 *
 * The final `result` message carries the full assistant text, which
 * agent-backend's `parseOutput` turns into summary + resolve flag.
 */

import type { FixProgressEvent, FixProgressEventKind } from '../../shared/types'
import { truncate } from '../../shared/annotation-utils'

export interface DescribedAgentMessage {
  event: FixProgressEvent
  finalText?: string
  /** Claude session id, carried on the init message so a thread can resume. */
  sessionId?: string
}

export function describeAgentMessage(payload: unknown): DescribedAgentMessage | null {
  if (!payload || typeof payload !== 'object') return null
  const message = payload as any
  const type = message.type as string | undefined

  if (type === 'system') {
    // The stream emits many `system` messages per run (init, status, hooks …),
    // all carrying the same session_id. Only the real init is worth surfacing;
    // the rest would render as a flood of identical "init <session_id>" lines.
    if (message.subtype !== 'init') return null
    const model = typeof message.model === 'string' ? message.model : 'session'
    const sessionId = typeof message.session_id === 'string' ? message.session_id : undefined
    return { ...makeEvent('system', `init ${model}`), sessionId }
  }

  if (type === 'assistant') {
    const content = message.message?.content
    if (!Array.isArray(content)) return null
    const blocks = content.map(describeContentBlock).filter(Boolean) as Array<{
      kind: FixProgressEventKind
      text: string
    }>
    if (blocks.length === 0) return null
    const merged = blocks.map((b) => b.text).join(' | ')
    const kind = blocks[0].kind
    return makeEvent(kind, merged)
  }

  if (type === 'user') {
    const content = message.message?.content
    if (!Array.isArray(content)) return null
    const results = content
      .filter((block: any) => block?.type === 'tool_result')
      .map((block: any) => summarizeToolResult(block))
      .filter((line: string | null): line is string => !!line)
    if (results.length === 0) return null
    return makeEvent('tool_result', results.join(' | '))
  }

  if (type === 'result') {
    const finalText: string = typeof message.result === 'string' ? message.result : ''
    const subtype = (message.subtype as string | undefined) ?? 'done'
    const summary = finalText
      ? truncate(finalText.split(/\r?\n/).filter((line: string) => line.trim()).pop() ?? '', 200)
      : subtype
    return { ...makeEvent('result', summary), finalText }
  }

  // Allowlist: only the types handled above are surfaced. Anything else
  // (status pings, rate limit events, hook lifecycle …) is dropped so new SDK
  // message types can't flood the panel. Movement is already visible through
  // assistant/tool_use/tool_result events.
  return null
}

function describeContentBlock(block: any): { kind: FixProgressEventKind; text: string } | null {
  if (!block || typeof block !== 'object') return null
  if (block.type === 'text') {
    const text = typeof block.text === 'string' ? block.text.trim() : ''
    if (!text) return null
    return { kind: 'text', text: truncate(text, 240) }
  }
  if (block.type === 'tool_use') {
    const name = typeof block.name === 'string' ? block.name : 'tool'
    const summary = summarizeToolInput(name, block.input)
    return { kind: 'tool_use', text: summary }
  }
  if (block.type === 'thinking') {
    const text = typeof block.thinking === 'string' ? block.thinking : ''
    if (!text) return null
    return { kind: 'text', text: `(thinking) ${truncate(text, 180)}` }
  }
  return null
}

function summarizeToolInput(name: string, input: unknown): string {
  if (!input || typeof input !== 'object') return name
  const record = input as Record<string, unknown>
  const hint =
    pickString(record, ['file_path', 'path', 'filePath']) ??
    pickString(record, ['command', 'cmd']) ??
    pickString(record, ['pattern', 'query']) ??
    pickString(record, ['url'])
  return hint ? `${name} ${truncate(hint, 160)}` : name
}

function summarizeToolResult(block: any): string | null {
  const content = block?.content
  if (typeof content === 'string') {
    const trimmed = content.trim()
    if (!trimmed) return block?.is_error ? 'tool error' : '(empty output)'
    return truncate((block.is_error ? 'tool error: ' : '') + trimmed.split(/\r?\n/)[0], 200)
  }
  if (Array.isArray(content)) {
    const parts: string[] = []
    for (const entry of content) {
      if (entry?.type === 'text' && typeof entry.text === 'string' && entry.text.trim()) {
        parts.push(truncate(entry.text.split(/\r?\n/)[0], 200))
      } else if (entry?.type === 'image') {
        const mime = typeof entry.source?.media_type === 'string'
          ? entry.source.media_type
          : (typeof entry.mimeType === 'string' ? entry.mimeType : 'image')
        parts.push(`image (${mime})`)
      }
    }
    if (parts.length) return parts.join(' · ')
  }
  return block?.is_error ? 'tool error' : 'tool result'
}

function pickString(record: Record<string, unknown>, keys: string[]): string | null {
  for (const key of keys) {
    const value = record[key]
    if (typeof value === 'string' && value.trim()) return value
  }
  return null
}

function makeEvent(kind: FixProgressEventKind, text: string): DescribedAgentMessage {
  return {
    event: {
      kind,
      text: truncate(text, 320),
      timestamp: new Date().toISOString(),
    },
  }
}
