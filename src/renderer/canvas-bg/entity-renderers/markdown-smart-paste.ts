import type { Extension } from '@codemirror/state'
import { EditorView } from '@codemirror/view'

interface PasteData {
  readonly types: readonly string[]
  getData(type: string): string
}

export interface DetectedContent {
  lang: string
  text: string
}

/**
 * Pretty-print `raw` as JSON, but only when it is an object or array — bare
 * primitives like `42` or `"x"` aren't worth a fenced block and a stray number
 * shouldn't trip detection. Returns null when not structured JSON. Shared by
 * the `application/json` MIME path and the plain-text sniff so both apply the
 * same bar.
 */
function tryFormatJson(raw: string): string | null {
  const trimmed = raw.trim()
  const isStructured =
    (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
    (trimmed.startsWith('[') && trimmed.endsWith(']'))
  if (!isStructured) return null
  try {
    return JSON.stringify(JSON.parse(trimmed), null, 2)
  } catch {
    return null
  }
}

/**
 * Returns formatted content + language tag when clipboard carries a strong
 * structural signal (MIME type identifies it, or it parses unambiguously).
 * Returns null when uncertain — bias is hard toward doing nothing so
 * mis-fires don't wrap prose in code fences.
 */
export function detectSmartPaste(data: PasteData): DetectedContent | null {
  const types = Array.from(data.types)

  // MIME hints: highest-confidence path
  if (types.includes('image/svg+xml')) {
    const text = (data.getData('image/svg+xml') || data.getData('text/plain')).trim()
    if (text) return { lang: 'svg', text }
  }

  if (types.includes('application/json')) {
    const raw = data.getData('application/json') || data.getData('text/plain')
    const json = raw ? tryFormatJson(raw) : null
    // malformed/primitive despite the MIME type — fall through to plain sniffs
    if (json) return { lang: 'json', text: json }
  }

  // Structural sniffs on plain text
  const plain = data.getData('text/plain')
  if (!plain) return null
  const trimmed = plain.trim()
  if (!trimmed) return null

  // SVG: must open with <svg (after an optional <?xml ...?> prologue) and close
  // with </svg> (complete document).
  if (/^(?:<\?xml[^>]*\?>\s*)?<svg[\s>]/i.test(trimmed) && /<\/svg\s*>$/i.test(trimmed)) {
    return { lang: 'svg', text: trimmed }
  }

  // JSON: objects and arrays only — not primitive values like "123" or "true"
  const json = tryFormatJson(trimmed)
  if (json) return { lang: 'json', text: json }

  // Full HTML document — not generic HTML snippets
  if (/^<!doctype\s+html/i.test(trimmed) || /^<html[\s>]/i.test(trimmed)) {
    return { lang: 'html', text: trimmed }
  }

  return null
}

/**
 * Wrap `text` in a fenced code block whose fence is one backtick longer than
 * the longest backtick run inside the content (min 3), per CommonMark — so
 * content that itself contains ``` can't close the block early.
 */
export function wrapInFence(lang: string, text: string): string {
  let longestRun = 0
  for (const match of text.matchAll(/`+/g)) {
    longestRun = Math.max(longestRun, match[0].length)
  }
  const fence = '`'.repeat(Math.max(3, longestRun + 1))
  return `${fence}${lang}\n${text}\n${fence}`
}

/**
 * CodeMirror extension: intercepts paste and wraps detected structured content
 * in a fenced code block. Falls through to normal paste when detection is
 * uncertain so plain paste is always available.
 */
export function smartPasteExtension(): Extension {
  return EditorView.domEventHandlers({
    paste(event, view) {
      const { clipboardData } = event
      if (!clipboardData) return false

      const detected = detectSmartPaste(clipboardData)
      if (!detected) return false

      event.preventDefault()
      const fenced = wrapInFence(detected.lang, detected.text)
      const { from, to } = view.state.selection.main
      view.dispatch({
        changes: { from, to, insert: fenced },
        selection: { anchor: from + fenced.length },
      })
      return true
    },
  })
}
