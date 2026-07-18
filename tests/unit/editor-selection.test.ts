import { describe, expect, it, vi } from 'vitest'
import {
  autofocusEditorSelection,
  focusAndSelectAll,
} from '../../src/shared/editor-selection'

describe('editor selection', () => {
  it('selects short text when its editor enters edit mode', () => {
    expect(autofocusEditorSelection(12, true)).toEqual({ anchor: 0, head: 12 })
  })

  it('places the cursor at the end when a document editor autofocuses', () => {
    expect(autofocusEditorSelection(12, false)).toEqual({
      anchor: 12,
      head: 12,
    })
  })

  it('focuses and selects all contents of a native text input', () => {
    const input = { focus: vi.fn(), select: vi.fn() }

    focusAndSelectAll(input)

    expect(input.focus).toHaveBeenCalledOnce()
    expect(input.select).toHaveBeenCalledOnce()
  })
})
