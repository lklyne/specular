/**
 * Pure parser for `claude -p --output-format stream-json --verbose` events.
 *
 * Each line from stdout is a JSON message following Claude Code's SDK schema.
 * We map it to a short, human-readable FixProgressEvent for the panel log.
 * The final `result` event carries the full assistant text, which we feed
 * back into claude-spawner's `parseOutput` to derive summary + resolve flag.
 */

import type { FixProgressEvent, FixProgressEventKind } from '../../shared/types'
import { truncate } from '../../shared/annotation-utils'

export interface ParsedStreamEvent {
  event: FixProgressEvent
  finalText?: string
  /** Claude Code session id, carried on the init event so a thread can resume. */
  sessionId?: string
}

export function parseStreamLine(line: string): ParsedStreamEvent | null {
  const trimmed = line.trim()
  if (!trimmed) return null
  let payload: any
  try {
    payload = JSON.parse(trimmed)
  } catch {
    return makeEvent('system', trimmed.slice(0, 200))
  }
  if (!payload || typeof payload !== 'object') return null

  const type = payload.type as string | undefined

  if (type === 'system') {
    // The stream emits many `system` messages per run (init, status, hooks …),
    // all carrying the same session_id. Only the real init is worth surfacing;
    // the rest would render as a flood of identical "init <session_id>" lines.
    if (payload.subtype !== 'init') return null
    const model = typeof payload.model === 'string' ? payload.model : 'session'
    const sessionId = typeof payload.session_id === 'string' ? payload.session_id : undefined
    return { ...makeEvent('system', `init ${model}`), sessionId }
  }

  if (type === 'assistant') {
    const content = payload.message?.content
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
    const content = payload.message?.content
    if (!Array.isArray(content)) return null
    const results = content
      .filter((block: any) => block?.type === 'tool_result')
      .map((block: any) => summarizeToolResult(block))
      .filter((line: string | null): line is string => !!line)
    if (results.length === 0) return null
    return makeEvent('tool_result', results.join(' | '))
  }

  if (type === 'result') {
    const finalText: string = typeof payload.result === 'string' ? payload.result : ''
    const subtype = (payload.subtype as string | undefined) ?? 'done'
    const summary = finalText
      ? truncate(finalText.split(/\r?\n/).filter((line: string) => line.trim()).pop() ?? '', 200)
      : subtype
    return { ...makeEvent('result', summary), finalText }
  }

  // Allowlist: only the types handled above are surfaced. Anything else
  // (rate_limit_event, status pings, future control messages) is dropped so
  // new CLI message types can't flood the panel. Movement is already visible
  // through assistant/tool_use/tool_result events.
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

function makeEvent(kind: FixProgressEventKind, text: string): ParsedStreamEvent {
  return {
    event: {
      kind,
      text: truncate(text, 320),
      timestamp: new Date().toISOString(),
    },
  }
}
