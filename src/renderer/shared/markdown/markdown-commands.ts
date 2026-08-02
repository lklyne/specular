/**
 * Markdown formatting commands (Cmd+B, Cmd+I, …) for the markdown editor.
 *
 * These edit the source text — `**bold**` — rather than applying rich-text
 * state, so the result is the same markdown a user could have typed and the
 * live-preview plugin collapses the markers on the next line change.
 *
 * Every command is a no-op on a read-only editor.
 */

import { syntaxTree } from '@codemirror/language'
import { EditorSelection, type EditorState, type StateCommand } from '@codemirror/state'
import type { KeyBinding } from '@codemirror/view'
import { BULLET_LINE } from './bullet-line'

/**
 * Markup that owns the head of a line. It is structure, not prose, so no
 * formatting command may put a marker in front of it — `**- item**` isn't a
 * bold list item, it's a paragraph that starts with a dash.
 */
const BLOCK_MARKUP = new Set(['ListMark', 'QuoteMark', 'HeaderMark', 'TaskMarker'])

/** First position on `pos`'s line that holds prose rather than block markup. */
function inlineStart(state: EditorState, pos: number): number {
  const line = state.doc.lineAt(pos)
  let start = line.from
  const skipSpaces = () => {
    while (start < line.to && state.sliceDoc(start, start + 1) === ' ') start += 1
  }
  skipSpaces()
  syntaxTree(state).iterate({
    from: line.from,
    to: line.to,
    enter: (node) => {
      if (!BLOCK_MARKUP.has(node.name)) return
      // Only markup in the unbroken run at the head of the line counts; a `-`
      // in the middle of a sentence is prose.
      if (node.from > start) return
      start = Math.max(start, node.to)
      skipSpaces()
    },
  })
  return Math.min(start, line.to)
}

/**
 * Narrow a selection to the prose inside it: block markup trimmed off the
 * front, whitespace off the back. Every command runs through this, so
 * selecting a whole list item and pressing Cmd+B bolds the item rather than
 * swallowing its bullet — no per-command special-casing.
 *
 * ponytail: trims the edges of the selection, so a selection spanning several
 * list items still wraps the whole span and leaves the inner markers inside.
 * Apply per line if that comes up.
 */
function inlineContentRange(
  state: EditorState,
  from: number,
  to: number,
): { from: number; to: number } {
  const start = Math.max(from, inlineStart(state, from))
  let end = Math.max(to, start)
  while (end > start && /\s/.test(state.sliceDoc(end - 1, end))) end -= 1
  return { from: start, to: end }
}

/**
 * Wrap the selection in `marker`, or strip it if it is already wrapped —
 * whether the markers sit just inside the selection (the user selected
 * `**bold**`) or just outside it (they selected `bold`).
 *
 * With an empty selection this inserts the pair and drops the cursor between
 * them, so Cmd+B then typing works like it does in a rich-text editor.
 */
export function toggleWrap(marker: string): StateCommand {
  return ({ state, dispatch }) => {
    if (state.readOnly) return false
    const width = marker.length
    const transaction = state.changeByRange((selection) => {
      const range = inlineContentRange(state, selection.from, selection.to)
      const outsideBefore = state.sliceDoc(range.from - width, range.from)
      const outsideAfter = state.sliceDoc(range.to, range.to + width)
      if (outsideBefore === marker && outsideAfter === marker) {
        return {
          changes: [
            { from: range.from - width, to: range.from },
            { from: range.to, to: range.to + width },
          ],
          range: EditorSelection.range(range.from - width, range.to - width),
        }
      }
      const selected = state.sliceDoc(range.from, range.to)
      if (
        selected.length >= width * 2 &&
        selected.startsWith(marker) &&
        selected.endsWith(marker)
      ) {
        return {
          changes: [
            { from: range.from, to: range.from + width },
            { from: range.to - width, to: range.to },
          ],
          range: EditorSelection.range(range.from, range.to - width * 2),
        }
      }
      return {
        changes: [
          { from: range.from, insert: marker },
          { from: range.to, insert: marker },
        ],
        range: EditorSelection.range(range.from + width, range.to + width),
      }
    })
    dispatch(state.update(transaction, { scrollIntoView: true, userEvent: 'input' }))
    return true
  }
}

/**
 * `[label](url)`. A selection that is already a URL becomes the target and the
 * cursor lands in the empty label; anything else becomes the label and the
 * cursor lands in the empty target. Either way the cursor is on the part the
 * user still has to type.
 */
export const insertLink: StateCommand = ({ state, dispatch }) => {
  if (state.readOnly) return false
  const transaction = state.changeByRange((selection) => {
    const range = inlineContentRange(state, selection.from, selection.to)
    const selected = state.sliceDoc(range.from, range.to)
    const selectionIsUrl = /^(?:https?:\/\/|www\.)\S+$/.test(selected)
    const label = selectionIsUrl ? '' : selected
    const url = selectionIsUrl ? selected : ''
    return {
      changes: { from: range.from, to: range.to, insert: `[${label}](${url})` },
      range: EditorSelection.cursor(
        selectionIsUrl ? range.from + 1 : range.from + label.length + 3,
      ),
    }
  })
  dispatch(state.update(transaction, { scrollIntoView: true, userEvent: 'input' }))
  return true
}

export const toggleBold = toggleWrap('**')
export const toggleItalic = toggleWrap('*')
export const toggleInlineCode = toggleWrap('`')
export const toggleStrikethrough = toggleWrap('~~')

/** Line numbers spanned by the selection, both endpoints' lines inclusive. */
export function selectedLineNumbers(state: EditorState): Set<number> {
  const lines = new Set<number>()
  for (const range of state.selection.ranges) {
    const first = state.doc.lineAt(range.from).number
    const last = state.doc.lineAt(range.to).number
    for (let n = first; n <= last; n += 1) lines.add(n)
  }
  return lines
}

/**
 * Toggle every non-blank line spanned by the selection between bulleted and
 * plain: strips the marker when every such line already has one, otherwise
 * adds one to whichever lines lack it (a mixed selection becomes fully
 * bulleted rather than fully stripped — "add" is the more common intent).
 */
export const toggleBulletList: StateCommand = ({ state, dispatch }) => {
  if (state.readOnly) return false

  const lines = [...selectedLineNumbers(state)]
    .sort((a, b) => a - b)
    .map((n) => state.doc.line(n))
  const nonEmpty = lines.filter((line) => line.text.trim().length > 0)
  if (nonEmpty.length === 0) return true
  const allBulleted = nonEmpty.every((line) => BULLET_LINE.test(line.text))

  const changes: Array<{ from: number; to?: number; insert: string }> = []
  for (const line of nonEmpty) {
    if (allBulleted) {
      const match = BULLET_LINE.exec(line.text)
      if (match) changes.push({ from: line.from, to: line.from + match[0].length, insert: '' })
    } else if (!BULLET_LINE.test(line.text)) {
      const indent = /^\s*/.exec(line.text)?.[0].length ?? 0
      changes.push({ from: line.from + indent, insert: '- ' })
    }
  }
  if (changes.length === 0) return true

  const changeSet = state.changes(changes)
  dispatch(
    state.update({
      changes: changeSet,
      selection: state.selection.map(changeSet),
      scrollIntoView: true,
      userEvent: 'input',
    }),
  )
  return true
}

/** Bind above defaultKeymap so these win where the two overlap. */
export const markdownFormattingKeymap: readonly KeyBinding[] = [
  { key: 'Mod-b', run: toggleBold, preventDefault: true },
  { key: 'Mod-i', run: toggleItalic, preventDefault: true },
  { key: 'Mod-e', run: toggleInlineCode, preventDefault: true },
  { key: 'Mod-Shift-x', run: toggleStrikethrough, preventDefault: true },
  { key: 'Mod-k', run: insertLink, preventDefault: true },
  { key: 'Mod-Shift-8', run: toggleBulletList, preventDefault: true },
]

/** Sticky notes support a smaller formatting surface: no inline code (Mod-e) or links (Mod-k). */
export const stickyFormattingKeymap: readonly KeyBinding[] = [
  { key: 'Mod-b', run: toggleBold, preventDefault: true },
  { key: 'Mod-i', run: toggleItalic, preventDefault: true },
  { key: 'Mod-Shift-x', run: toggleStrikethrough, preventDefault: true },
  { key: 'Mod-Shift-8', run: toggleBulletList, preventDefault: true },
]
