import { describe, expect, it, vi } from 'vitest'
import { commitInlineEditBeforePointerAction } from '../../src/renderer/above-view/useCanvasPointerRouter'

describe('commitInlineEditBeforePointerAction', () => {
  it('blurs the active editor before closing main edit mode', () => {
    const calls: string[] = []
    const blurActiveEditor = vi.fn(() => calls.push('blur'))
    const commitEntityEdit = vi.fn(() => calls.push('commit'))

    commitInlineEditBeforePointerAction(blurActiveEditor, commitEntityEdit)

    expect(calls).toEqual(['blur', 'commit'])
  })
})
