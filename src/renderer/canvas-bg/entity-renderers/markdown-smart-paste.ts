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
    if (raw) {
      try {
        return { lang: 'json', text: JSON.stringify(JSON.parse(raw), null, 2) }
      } catch {
        // malformed despite the MIME type — fall through to plain text sniffs
      }
    }
  }

  // Structural sniffs on plain text
  const plain = data.getData('text/plain')
  if (!plain) return null
  const trimmed = plain.trim()
  if (!trimmed) return null

  // SVG: must open with <svg and close with </svg> (complete document)
  if (/^<svg[\s>]/i.test(trimmed) && /<\/svg\s*>$/i.test(trimmed)) {
    return { lang: 'svg', text: trimmed }
  }

  // JSON: objects and arrays only — not primitive values like "123" or "true"
  if (
    (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
    (trimmed.startsWith('[') && trimmed.endsWith(']'))
  ) {
    try {
      return { lang: 'json', text: JSON.stringify(JSON.parse(trimmed), null, 2) }
    } catch {
      // not valid JSON
    }
  }

  // Full HTML document — not generic HTML snippets
  if (/^<!doctype\s+html/i.test(trimmed) || /^<html[\s>]/i.test(trimmed)) {
    return { lang: 'html', text: trimmed }
  }

  return null
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
      const fenced = `\`\`\`${detected.lang}\n${detected.text}\n\`\`\``
      const { from, to } = view.state.selection.main
      view.dispatch({
        changes: { from, to, insert: fenced },
        selection: { anchor: from + fenced.length },
      })
      return true
    },
  })
}
