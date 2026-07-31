import { describe, expect, it } from 'vitest'
import { clipboardOwner } from '../../src/shared/clipboard-owner'

describe('clipboardOwner', () => {
  it('leaves an already-handled editor paste with the native editor', () => {
    expect(
      clipboardOwner({
        eventAlreadyHandled: true,
        inlineEditorActive: true,
        typingSurfaceInEventPath: false,
        hasNativeSelection: false,
      }),
    ).toBe('native')
  })
})
