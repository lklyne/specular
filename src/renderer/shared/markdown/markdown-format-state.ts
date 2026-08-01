/**
 * Derives the sticky formatting popover's toggle state from the selection
 * head — which marks are active there, syntax-tree ancestors first, then
 * whether the head's line is a bullet-list item. Pure so it's cheap to run
 * on every cursor move and easy to unit-test without mounting an editor.
 */

import { syntaxTree } from '@codemirror/language'
import type { EditorState } from '@codemirror/state'

export interface StickyFormatState {
  bold: boolean
  italic: boolean
  strikethrough: boolean
  bulletList: boolean
}

const BULLET_LINE = /^\s*[-*+] /

export function selectionFormatState(state: EditorState): StickyFormatState {
  const head = state.selection.main.head

  let bold = false
  let italic = false
  let strikethrough = false
  let node = syntaxTree(state).resolveInner(head, -1)
  while (node) {
    if (node.name === 'StrongEmphasis') bold = true
    if (node.name === 'Emphasis') italic = true
    if (node.name === 'Strikethrough') strikethrough = true
    const parent = node.parent
    if (!parent) break
    node = parent
  }

  const line = state.doc.lineAt(head)
  const bulletList = BULLET_LINE.test(line.text)

  return { bold, italic, strikethrough, bulletList }
}
