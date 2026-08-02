import { describe, expect, it } from 'vitest'
import { EditorState, type TransactionSpec } from '@codemirror/state'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import {
  continueBulletOnEnter,
  removeBulletPrefixOnBackspace,
  unwrapMarkAtBoundary,
} from '../../src/renderer/shared/markdown/sticky-editing'

/**
 * Run a pure `(state) => TransactionSpec | null` helper with the caret marked
 * by `|` and return the result in the same notation, or the input unchanged
 * (with `|` restored) when the helper returns null.
 */
function run(
  helper: (state: EditorState) => TransactionSpec | null,
  marked: string,
): string {
  const cursor = marked.indexOf('|')
  const doc = marked.replace('|', '')
  const state = EditorState.create({
    doc,
    selection: { anchor: cursor },
    extensions: [markdown({ base: markdownLanguage })],
  })
  const spec = helper(state)
  if (!spec) return marked
  const next = state.update(spec)
  const { from, to } = next.state.selection.main
  const text = next.state.doc.toString()
  return from === to
    ? `${text.slice(0, from)}|${text.slice(from)}`
    : `${text.slice(0, from)}|${text.slice(from, to)}|${text.slice(to)}`
}

function backspace(state: EditorState): TransactionSpec | null {
  return unwrapMarkAtBoundary(state, state.selection.main.head, 'before')
}

function del(state: EditorState): TransactionSpec | null {
  return unwrapMarkAtBoundary(state, state.selection.main.head, 'after')
}

describe('unwrapMarkAtBoundary', () => {
  it('Backspace right after **bold** unwraps, caret at the end of the text', () => {
    expect(run(backspace, '**bold**|')).toBe('bold|')
  })

  it('Backspace at the start of the content inside the span unwraps', () => {
    // Unwrapping always drops the caret at the end of the freed text —
    // Backspace's "end", regardless of which boundary triggered it.
    expect(run(backspace, '**|bold**')).toBe('bold|')
  })

  it('Delete right before the opening marker unwraps', () => {
    expect(run(del, '|**bold**')).toBe('|bold')
  })

  it('does nothing mid-word', () => {
    expect(run(backspace, '**bo|ld**')).toBe('**bo|ld**')
  })

  it('deleting the sole content char removes the whole span, not just the char', () => {
    expect(run(backspace, '**b|**')).toBe('|')
  })

  it('handles *italic* the same way', () => {
    expect(run(backspace, '*x*|')).toBe('x|')
  })

  it('handles ~~strikethrough~~ the same way', () => {
    expect(run(backspace, '~~x~~|')).toBe('x|')
  })

  it('returns null outside any styled span', () => {
    expect(run(backspace, 'plain text|')).toBe('plain text|')
  })

  it('returns null on a non-empty selection', () => {
    const state = EditorState.create({
      doc: '**bold**',
      selection: { anchor: 0, head: 4 },
      extensions: [markdown({ base: markdownLanguage })],
    })
    expect(unwrapMarkAtBoundary(state, 4, 'before')).toBeNull()
  })
})

/** Run a keystroke helper over `doc` at `cursor` and return the resulting doc text, or null. */
function docAfter(
  helper: (state: EditorState) => TransactionSpec | null,
  doc: string,
  cursor: number,
): string | null {
  const state = EditorState.create({
    doc,
    selection: { anchor: cursor },
    extensions: [markdown({ base: markdownLanguage })],
  })
  const spec = helper(state)
  if (!spec) return null
  return state.update(spec).state.doc.toString()
}

describe('continueBulletOnEnter', () => {
  it('continues a bullet at the same indent', () => {
    expect(docAfter(continueBulletOnEnter, '  - milk', 8)).toBe('  - milk\n  - ')
  })

  it('exits the list on an empty item, removing the marker', () => {
    expect(docAfter(continueBulletOnEnter, '- milk\n- ', 9)).toBe('- milk\n')
  })

  it('does nothing on a non-list line', () => {
    expect(continueBulletOnEnter(
      EditorState.create({
        doc: 'plain text',
        selection: { anchor: 10 },
        extensions: [markdown({ base: markdownLanguage })],
      }),
    )).toBeNull()
  })
})

describe('removeBulletPrefixOnBackspace', () => {
  it('strips the prefix when the caret sits right after it', () => {
    expect(docAfter(removeBulletPrefixOnBackspace, '- milk', 2)).toBe('milk')
  })

  it('does nothing elsewhere on the line', () => {
    expect(docAfter(removeBulletPrefixOnBackspace, '- milk', 4)).toBeNull()
  })

  it('does nothing on a non-list line', () => {
    expect(docAfter(removeBulletPrefixOnBackspace, 'plain text', 3)).toBeNull()
  })
})
