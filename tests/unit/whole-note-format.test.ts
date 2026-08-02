import { describe, expect, it } from 'vitest'
import {
  toggleWholeNoteBullets,
  toggleWholeNoteWrap,
  wholeNoteFormatState,
} from '../../src/renderer/shared/markdown/whole-note-format'

describe('toggleWholeNoteWrap', () => {
  it('wraps every non-empty line, skipping blanks', () => {
    expect(toggleWholeNoteWrap('one\n\ntwo', '**')).toBe('**one**\n\n**two**')
  })

  it('wraps line content after a bullet prefix, not the marker', () => {
    expect(toggleWholeNoteWrap('- milk\n- eggs', '**')).toBe('- **milk**\n- **eggs**')
  })

  it('unwraps when every line is already wrapped', () => {
    expect(toggleWholeNoteWrap('**one**\n- **two**', '**')).toBe('one\n- two')
  })

  it('mixed lines get wrapped, absorbing partial marks', () => {
    expect(toggleWholeNoteWrap('has **bold** inside\nplain', '**')).toBe(
      '**has bold inside**\n**plain**',
    )
  })

  it('two separate bold spans on one line is not "already wrapped"', () => {
    expect(toggleWholeNoteWrap('**a** and **b**', '**')).toBe('**a and b**')
  })

  it('strikethrough stacks around existing whole-line bold', () => {
    expect(toggleWholeNoteWrap('**one**', '~~')).toBe('**~~one~~**')
  })

  it('unwrapping bold keeps a strikethrough layer', () => {
    expect(toggleWholeNoteWrap('**~~one~~**', '**')).toBe('~~one~~')
  })

  it('no-ops on empty text', () => {
    expect(toggleWholeNoteWrap('', '**')).toBe('')
    expect(toggleWholeNoteWrap('\n\n', '~~')).toBe('\n\n')
  })
})

describe('toggleWholeNoteBullets', () => {
  it('bullets every non-empty line', () => {
    expect(toggleWholeNoteBullets('one\n\ntwo')).toBe('- one\n\n- two')
  })

  it('un-bullets when all lines are bulleted', () => {
    expect(toggleWholeNoteBullets('- one\n- two')).toBe('one\ntwo')
  })

  it('mixed lines all become bulleted', () => {
    expect(toggleWholeNoteBullets('- one\ntwo')).toBe('- one\n- two')
  })
})

describe('wholeNoteFormatState', () => {
  it('is all-false for plain text and empty notes', () => {
    expect(wholeNoteFormatState('hello\nworld')).toEqual({
      bold: false,
      strikethrough: false,
      bulletList: false,
    })
    expect(wholeNoteFormatState('')).toEqual({
      bold: false,
      strikethrough: false,
      bulletList: false,
    })
  })

  it('detects whole-note bold across bullets and nesting order', () => {
    expect(wholeNoteFormatState('**one**\n- ~~**two**~~').bold).toBe(true)
    expect(wholeNoteFormatState('**one**\ntwo').bold).toBe(false)
  })

  it('partial marks on a line do not count as whole-line bold', () => {
    expect(wholeNoteFormatState('**a** and **b**').bold).toBe(false)
  })

  it('detects strikethrough and bullet list', () => {
    const state = wholeNoteFormatState('- ~~one~~\n- ~~two~~')
    expect(state.strikethrough).toBe(true)
    expect(state.bulletList).toBe(true)
  })
})
