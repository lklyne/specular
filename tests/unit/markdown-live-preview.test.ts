import { describe, expect, it } from 'vitest'
import { EditorState } from '@codemirror/state'
import { EditorView } from '@codemirror/view'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import {
  FULL_LIVE_PREVIEW,
  STICKY_LIVE_PREVIEW,
  type LivePreviewOptions,
  buildLinkMarks,
  buildMarkdownDecorations,
} from '../../src/renderer/shared/markdown/markdown-live-preview'

/** What the reader sees: source minus every collapsed range. */
function visibleText(
  doc: string,
  editable: boolean,
  cursor?: number,
  options: LivePreviewOptions = FULL_LIVE_PREVIEW,
): string {
  const state = EditorState.create({
    doc,
    selection: cursor === undefined ? undefined : { anchor: cursor },
    extensions: [markdown({ base: markdownLanguage }), EditorView.editable.of(editable)],
  })
  const decorations = buildMarkdownDecorations(state, [{ from: 0, to: doc.length }], options)
  const cuts: Array<{ from: number; to: number; widget: string }> = []
  decorations.between(0, doc.length, (from, to, value) => {
    if (from === to && !value.spec.widget) return
    const widget = value.spec.widget
    cuts.push({
      from,
      to,
      widget: widget && 'text' in widget ? String((widget as { text: string }).text) : '',
    })
  })
  let out = ''
  let cursorPos = 0
  for (const cut of cuts.sort((a, b) => a.from - b.from)) {
    if (cut.from < cursorPos) continue
    out += doc.slice(cursorPos, cut.from) + cut.widget
    cursorPos = cut.to
  }
  return out + doc.slice(cursorPos)
}

describe('markdown live preview', () => {
  it('collapses inline punctuation when the editor is read-only', () => {
    expect(visibleText('**bold** and *soft*', false)).toBe('bold and soft')
  })

  it('keeps a bullet glyph so list items stay legible without the dash', () => {
    expect(visibleText('- milk\n- eggs', false)).toBe('• milk\n• eggs')
  })

  it('leaves ordered markers alone — the number is the content', () => {
    expect(visibleText('1. first\n2. second', false)).toBe('1. first\n2. second')
  })

  it('collapses a heading to its text, trailing space included', () => {
    expect(visibleText('## Notes', false)).toBe('Notes')
  })

  it('shows only the label of an inline link', () => {
    expect(visibleText('see [the docs](https://example.com)', false)).toBe('see the docs')
  })

  it('reveals markup on the line holding the cursor, and only that line', () => {
    expect(visibleText('**one**\n**two**', true, 2)).toBe('**one**\ntwo')
  })

  it('reveals nothing when the editor is not editable, cursor or not', () => {
    expect(visibleText('**one**\n**two**', false, 2)).toBe('one\ntwo')
  })
})

describe('sticky live preview', () => {
  it('renders a heading as literal text, editable or not', () => {
    expect(visibleText('# Title', true, 0, STICKY_LIVE_PREVIEW)).toBe('# Title')
    expect(visibleText('# Title', false, undefined, STICKY_LIVE_PREVIEW)).toBe('# Title')
  })

  it('renders a blockquote as literal text', () => {
    expect(visibleText('> quoted', false, undefined, STICKY_LIVE_PREVIEW)).toBe('> quoted')
  })

  it('renders a horizontal rule as literal text', () => {
    expect(visibleText('---', false, undefined, STICKY_LIVE_PREVIEW)).toBe('---')
  })

  it('renders a task marker as a glyph, not literal', () => {
    expect(visibleText('- [ ] task', false, undefined, STICKY_LIVE_PREVIEW)).toBe('• [ ] task')
  })

  it('keeps a bullet glyph for a plain list item', () => {
    expect(visibleText('- milk', false, undefined, STICKY_LIVE_PREVIEW)).toBe('• milk')
  })

  it('collapses bold and strikethrough when read-only', () => {
    expect(visibleText('**bold** ~~gone~~', false, undefined, STICKY_LIVE_PREVIEW)).toBe(
      'bold gone',
    )
  })

  it('never reveals bold/strikethrough markup, even with the cursor on that line', () => {
    expect(visibleText('**bold** ~~gone~~', true, 4, STICKY_LIVE_PREVIEW)).toBe('bold gone')
  })
})

/** Every link mark in the doc as (covered text, target URL) pairs. */
function linkMarks(doc: string): Array<{ text: string; url: string }> {
  const state = EditorState.create({
    doc,
    extensions: [markdown({ base: markdownLanguage })],
  })
  const set = buildLinkMarks(state, [{ from: 0, to: doc.length }])
  const out: Array<{ text: string; url: string }> = []
  set.between(0, doc.length, (from, to, value) => {
    out.push({
      text: doc.slice(from, to),
      url: String(value.spec.attributes?.['data-md-url']),
    })
  })
  return out
}

describe('markdown link target marks', () => {
  it('marks an inline link with its target URL', () => {
    expect(linkMarks('see [the docs](https://example.com)')).toEqual([
      { text: '[the docs](https://example.com)', url: 'https://example.com' },
    ])
  })

  it('marks a bare autolinked URL with itself', () => {
    expect(linkMarks('go to https://example.com now')).toEqual([
      { text: 'https://example.com', url: 'https://example.com' },
    ])
  })

  it('emits one mark per link, not a nested one for the URL child', () => {
    expect(linkMarks('[a](https://a.com) and [b](https://b.com)')).toHaveLength(2)
  })

  it('ignores targets that cannot become pages (relative paths, other schemes)', () => {
    expect(linkMarks('see [local](./notes.md) or [mail](mailto:x@y.com)')).toEqual([])
  })

  it('is unaffected by the sticky/full live-preview split — links stay clickable either way', () => {
    expect(linkMarks('see [the docs](https://example.com)')).toEqual([
      { text: '[the docs](https://example.com)', url: 'https://example.com' },
    ])
  })
})
