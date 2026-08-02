import { describe, expect, it } from 'vitest'
import { EditorState } from '@codemirror/state'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import { selectionFormatState } from '../../src/renderer/shared/markdown/markdown-format-state'

/** Build editor state for `marked` with the cursor at `|`. */
function stateAt(marked: string): EditorState {
  const anchor = marked.indexOf('|')
  const doc = marked.replace('|', '')
  return EditorState.create({
    doc,
    selection: { anchor },
    extensions: [markdown({ base: markdownLanguage })],
  })
}

describe('selectionFormatState', () => {
  it('is bold inside **x**', () => {
    expect(selectionFormatState(stateAt('**bo|ld**'))).toEqual({
      bold: true,
      italic: false,
      strikethrough: false,
      bulletList: false,
    })
  })

  it('is italic inside *x*', () => {
    expect(selectionFormatState(stateAt('*ita|lic*'))).toEqual({
      bold: false,
      italic: true,
      strikethrough: false,
      bulletList: false,
    })
  })

  it('is strikethrough inside ~~x~~', () => {
    expect(selectionFormatState(stateAt('~~str|uck~~'))).toEqual({
      bold: false,
      italic: false,
      strikethrough: true,
      bulletList: false,
    })
  })

  it('is bulletList on a `- item` line', () => {
    expect(selectionFormatState(stateAt('- ite|m'))).toEqual({
      bold: false,
      italic: false,
      strikethrough: false,
      bulletList: true,
    })
  })

  it('is all-false on plain text', () => {
    expect(selectionFormatState(stateAt('plain te|xt'))).toEqual({
      bold: false,
      italic: false,
      strikethrough: false,
      bulletList: false,
    })
  })
})
