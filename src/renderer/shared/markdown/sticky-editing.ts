/**
 * Editing commands for the sticky-note markdown variant.
 *
 * Sticky notes hide bold/italic/strikethrough markers unconditionally — they
 * never reveal on the cursor's line the way the full editor's do (see
 * `STICKY_LIVE_PREVIEW` in markdown-live-preview.ts). That means Backspace
 * and Delete can land the caret right next to a marker the user can't see,
 * so a plain character-delete would peel off one side of a pair and leave a
 * stray `**` behind. `unwrapMarkAtBoundary` intercepts that case and removes
 * the whole pair instead.
 *
 * Bullets follow the same "always collapsed" rule, so Enter/Backspace get
 * their own continuation logic rather than `@codemirror/lang-markdown`'s
 * `markdownKeymap` — that keymap also auto-continues quotes and ordered
 * lists, neither of which sticky notes support as live markup.
 */

import { syntaxTree } from '@codemirror/language'
import type { EditorState, TransactionSpec } from '@codemirror/state'
import type { KeyBinding } from '@codemirror/view'
import type { SyntaxNode } from '@lezer/common'
import { BULLET_LINE } from './bullet-line'

/** Node names of the styled spans sticky notes still format. */
const STYLED_NAMES = new Set(['Emphasis', 'StrongEmphasis', 'Strikethrough'])

/** Node names of the (always-hidden) marker pairs that wrap those spans. */
const MARK_NAMES = new Set(['EmphasisMark', 'StrikethroughMark'])

/**
 * Backspace (`side: 'before'`) or Delete (`side: 'after'`) at `pos` against a
 * hidden emphasis/strong/strikethrough marker. Returns a transaction that
 * unwraps the whole pair — deleting both markers and leaving the content
 * behind — or `null` when `pos` isn't adjacent to one of those markers, in
 * which case the caller should fall through to normal character deletion.
 *
 * Only applies to an empty selection: a ranged selection has its own delete
 * semantics and isn't a "boundary" in the sense this handles.
 */
export function unwrapMarkAtBoundary(
  state: EditorState,
  pos: number,
  side: 'before' | 'after',
): TransactionSpec | null {
  if (!state.selection.main.empty) return null

  // The start of the character this keypress would delete under default
  // handling — always resolved forward (`side: 1`), because we want the
  // node that *starts* at (or contains) that character regardless of which
  // key triggered the check.
  const biasPos = side === 'before' ? pos - 1 : pos
  if (side === 'before' && biasPos < 0) return null
  if (side === 'after' && biasPos >= state.doc.length) return null

  const node = syntaxTree(state).resolveInner(biasPos, 1)

  let styled: SyntaxNode | null = null
  let viaMark = false
  if (MARK_NAMES.has(node.name) && node.parent && STYLED_NAMES.has(node.parent.name)) {
    styled = node.parent
    viaMark = true
  } else if (STYLED_NAMES.has(node.name)) {
    // `biasPos` sits in the span's content, not a marker.
    styled = node
  } else {
    return null
  }

  const marks: Array<{ from: number; to: number }> = []
  for (let child = styled.firstChild; child; child = child.nextSibling) {
    if (MARK_NAMES.has(child.name)) marks.push({ from: child.from, to: child.to })
  }
  if (marks.length < 2) return null
  const open = marks[0]
  const close = marks[marks.length - 1]
  const contentFrom = open.to
  const contentTo = close.from
  if (contentTo <= contentFrom) return null

  if (!viaMark) {
    // Deleting the one remaining content char would leave the bare markers
    // (`****`) behind — remove the whole span instead.
    const soleChar = contentTo - contentFrom === 1
    if (!soleChar) return null
    return {
      changes: { from: styled.from, to: styled.to },
      selection: { anchor: styled.from },
    }
  }

  const openLen = open.to - open.from
  const contentStartNew = open.from
  const contentEndNew = contentTo - openLen
  return {
    changes: [
      { from: open.from, to: open.to },
      { from: close.from, to: close.to },
    ],
    selection: { anchor: side === 'before' ? contentEndNew : contentStartNew },
  }
}

export const stickyDeleteKeymap: readonly KeyBinding[] = [
  {
    key: 'Backspace',
    run: (view) => {
      const spec = unwrapMarkAtBoundary(view.state, view.state.selection.main.head, 'before')
      if (!spec) return false
      view.dispatch(view.state.update(spec, { userEvent: 'delete' }))
      return true
    },
  },
  {
    key: 'Delete',
    run: (view) => {
      const spec = unwrapMarkAtBoundary(view.state, view.state.selection.main.head, 'after')
      if (!spec) return false
      view.dispatch(view.state.update(spec, { userEvent: 'delete' }))
      return true
    },
  },
]

/**
 * Enter on a bullet line continues the list at the same indent, or — on an
 * empty item — removes the marker and exits the list, the same "second
 * Enter breaks out" convention as other outliners.
 */
export function continueBulletOnEnter(state: EditorState): TransactionSpec | null {
  const range = state.selection.main
  const line = state.doc.lineAt(range.head)
  const match = BULLET_LINE.exec(line.text)
  if (!match) return null

  if (line.text.length === match[0].length) {
    return {
      changes: { from: line.from, to: line.to, insert: '' },
      selection: { anchor: line.from },
    }
  }

  const indent = match[1]
  const insert = `\n${indent}- `
  return {
    changes: { from: range.from, to: range.to, insert },
    selection: { anchor: range.from + insert.length },
  }
}

/** Backspace right after a bullet's `- ` strips the prefix instead of merging lines. */
export function removeBulletPrefixOnBackspace(state: EditorState): TransactionSpec | null {
  if (!state.selection.main.empty) return null
  const pos = state.selection.main.head
  const line = state.doc.lineAt(pos)
  const match = BULLET_LINE.exec(line.text)
  if (!match) return null
  const markerEnd = line.from + match[0].length
  if (pos !== markerEnd) return null
  return {
    changes: { from: line.from, to: markerEnd, insert: '' },
    selection: { anchor: line.from },
  }
}

export const stickyListKeymap: readonly KeyBinding[] = [
  {
    key: 'Enter',
    run: (view) => {
      const spec = continueBulletOnEnter(view.state)
      if (!spec) return false
      view.dispatch(view.state.update(spec, { userEvent: 'input' }))
      return true
    },
  },
  {
    key: 'Backspace',
    run: (view) => {
      const spec = removeBulletPrefixOnBackspace(view.state)
      if (!spec) return false
      view.dispatch(view.state.update(spec, { userEvent: 'delete' }))
      return true
    },
  },
]
