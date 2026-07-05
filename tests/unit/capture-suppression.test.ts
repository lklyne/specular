import { describe, expect, it } from 'vitest'
import {
  CAPTURE_SUPPRESSION_CSS,
  CAPTURE_SUPPRESSION_STYLE_ID,
} from '../../src/preload/capture-suppression'

describe('capture suppression selectors', () => {
  it('hides page-injected inspection and comment overlays during bitmap capture', () => {
    expect(CAPTURE_SUPPRESSION_STYLE_ID).toBe('__canvas-capture-suppression')
    // Prefix-based, so new overlays following the id convention are covered
    // without editing this constant.
    expect(CAPTURE_SUPPRESSION_CSS).toContain('[id^="__canvas-dom-inspection-"]')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('[id^="__canvas-comment-"]')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('display: none !important')
  })
})
