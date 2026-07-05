import { describe, expect, it } from 'vitest'
import {
  CAPTURE_SUPPRESSION_CSS,
  CAPTURE_SUPPRESSION_STYLE_ID,
} from '../../src/preload/capture-suppression'

describe('capture suppression selectors', () => {
  it('hides page-injected inspection and comment overlays during bitmap capture', () => {
    expect(CAPTURE_SUPPRESSION_STYLE_ID).toBe('__canvas-capture-suppression')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('#__canvas-dom-inspection-label')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('[id^="__canvas-dom-inspection-margin-"]')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('[id^="__canvas-dom-inspection-padding-"]')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('[id^="__canvas-dom-inspection-gap-"]')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('#__canvas-comment-preview-layer')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('#__canvas-comment-hover-summary')
    expect(CAPTURE_SUPPRESSION_CSS).toContain('display: none !important')
  })
})
