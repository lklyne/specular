import { describe, expect, it } from 'vitest'
import { EditorState, type StateCommand, type Transaction } from '@codemirror/state'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import { markdownCommandsForTest } from '../../src/renderer/canvas-bg/entity-renderers/markdown-commands'

const { toggleWrap, insertLink } = markdownCommandsForTest

/**
 * Run a command over `doc` with the selection marked by `|` (one for a
 * cursor, two for a range) and return the result in the same notation.
 */
function run(command: StateCommand, marked: string, readOnly = false): string {
  const anchor = marked.indexOf('|')
  const head = marked.indexOf('|', anchor + 1)
  const doc = marked.replace(/\|/g, '')
  const state = EditorState.create({
    doc,
    selection: { anchor, head: head === -1 ? anchor : head - 1 },
    extensions: [
      markdown({ base: markdownLanguage }),
      ...(readOnly ? [EditorState.readOnly.of(true)] : []),
    ],
  })
  let next = state
  const handled = command({
    state,
    dispatch: (transaction: Transaction) => {
      next = transaction.state
    },
  })
  if (!handled) return marked
  const { from, to } = next.selection.main
  const text = next.doc.toString()
  return from === to
    ? `${text.slice(0, from)}|${text.slice(from)}`
    : `${text.slice(0, from)}|${text.slice(from, to)}|${text.slice(to)}`
}

const bold = toggleWrap('**')

describe('markdown formatting commands', () => {
  it('wraps the selection, leaving the words selected and the markers outside', () => {
    expect(run(bold, 'make |this| bold')).toBe('make **|this|** bold')
  })

  it('round-trips: pressing it twice restores the original text and selection', () => {
    expect(run(bold, run(bold, 'make |this| bold'))).toBe('make |this| bold')
  })

  it('unwraps when the markers sit inside the selection', () => {
    expect(run(bold, 'make |**this**| bold')).toBe('make |this| bold')
  })

  it('unwraps when the markers sit just outside the selection', () => {
    expect(run(bold, 'make **|this|** bold')).toBe('make |this| bold')
  })

  it('drops the cursor between a fresh pair when nothing is selected', () => {
    expect(run(bold, 'type |here')).toBe('type **|**here')
  })

  it('leaves a read-only document untouched', () => {
    expect(run(bold, 'make |this| bold', true)).toBe('make |this| bold')
  })

  it('puts the cursor in the empty target when linking plain text', () => {
    expect(run(insertLink, 'see |the docs|')).toBe('see [the docs](|)')
  })

  it('puts the cursor in the empty label when the selection is a URL', () => {
    expect(run(insertLink, 'see |https://example.com|')).toBe('see [|](https://example.com)')
  })
})

describe('formatting never swallows block markup', () => {
  it('bolds a list item without eating its bullet', () => {
    expect(run(bold, '|- milk|')).toBe('- **|milk|**')
  })

  it('bolds a numbered item without eating its number', () => {
    expect(run(bold, '|1. milk|')).toBe('1. **|milk|**')
  })

  it('bolds a quoted task without eating the quote or the checkbox', () => {
    expect(run(bold, '|> - [ ] milk|')).toBe('> - [ ] **|milk|**')
  })

  it('bolds a heading without eating its hashes', () => {
    expect(run(bold, '|## Notes|')).toBe('## **|Notes|**')
  })

  it('leaves trailing whitespace outside the markers', () => {
    expect(run(bold, '|- milk   |')).toBe('- **|milk|**   ')
  })

  it('moves a cursor parked before the bullet to the start of the prose', () => {
    expect(run(bold, '|- milk')).toBe('- **|**milk')
  })

  it('treats a dash inside a sentence as prose, not a bullet', () => {
    expect(run(bold, '|a - b|')).toBe('**|a - b|**')
  })

  it('links a list item without eating its bullet', () => {
    expect(run(insertLink, '|- the docs|')).toBe('- [the docs](|)')
  })
})
