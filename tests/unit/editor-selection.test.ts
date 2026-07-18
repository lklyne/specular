import { describe, expect, it, vi } from 'vitest'
import {
  autofocusEditorSelection,
  focusAndSelectAll,
} from '../../src/shared/editor-selection'

describe('editor selection', () => {
  it('selects the full document when autofocus enters edit mode', () => {
    expect(autofocusEditorSelection(12)).toEqual({ anchor: 0, head: 12 })
  })

  it('focuses and selects all contents of a native text input', () => {
    const input = { focus: vi.fn(), select: vi.fn() }

    focusAndSelectAll(input)

    expect(input.focus).toHaveBeenCalledOnce()
    expect(input.select).toHaveBeenCalledOnce()
  })
})
