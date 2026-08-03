import type { Extension } from '@codemirror/state'
import { EditorView } from '@codemirror/view'
import {
  detectStructuredContent,
  wrapInFence,
} from '../../../shared/structured-content'

export { wrapInFence }

/**
 * Returns formatted content + language tag when clipboard carries a strong
 * structural signal (MIME type identifies it, or it parses unambiguously).
 * Returns null when uncertain — bias is hard toward doing nothing so
 * mis-fires don't wrap prose in code fences.
 */
export const detectSmartPaste = detectStructuredContent

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
