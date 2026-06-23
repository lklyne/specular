import { describe, expect, it } from 'vitest'
import {
  agentOverlayClipInsets,
  agentOverlayClipPath,
} from '../../src/shared/agent-overlay-clip'

describe('agent overlay clip', () => {
  it('does not clip top or right because native overlay bounds already exclude them', () => {
    expect(agentOverlayClipInsets({ leftChromeWidth: 180 })).toEqual({
      top: 0,
      right: 0,
      bottom: 0,
      left: 180,
    })
  })

  it('keeps the CSS clip path local to the overlay window', () => {
    expect(agentOverlayClipPath({ leftChromeWidth: 0 })).toBe('inset(0px 0px 0px 0px)')
    expect(agentOverlayClipPath({ leftChromeWidth: 240 })).toBe('inset(0px 0px 0px 240px)')
  })
})
