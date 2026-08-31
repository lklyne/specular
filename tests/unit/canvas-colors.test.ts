import { describe, expect, it } from 'vitest'
import {
  CANVAS_COLOR_SLOTS,
  NEUTRAL_STORAGE,
  paletteSlots,
  resolveCanvasColor,
  slotForStorage,
} from '../../src/shared/canvas-colors'

describe('canvas-colors palette', () => {
  it('exposes the eight ADR-0013 slots in canonical order', () => {
    expect(CANVAS_COLOR_SLOTS.map((s) => s.id)).toEqual([
      'neutral',
      'purple',
      'blue',
      'cyan',
      'green',
      'yellow',
      'orange',
      'red',
    ])
  })

  it('neutral has no preset/hex; hues carry a preset and a distinct soft/vivid hex', () => {
    const neutral = CANVAS_COLOR_SLOTS.find((s) => s.id === 'neutral')!
    expect(neutral.preset).toBeNull()
    expect(neutral.soft).toBeNull()
    expect(neutral.vivid).toBeNull()
    for (const slot of CANVAS_COLOR_SLOTS.filter((s) => s.id !== 'neutral')) {
      expect(slot.preset).toMatch(/^[1-7]$/)
      expect(slot.soft).toMatch(/^#[0-9a-fA-F]{6}$/)
      expect(slot.vivid).toMatch(/^#[0-9a-fA-F]{6}$/)
      expect(slot.soft).not.toBe(slot.vivid)
    }
  })

  it('maps the six spec hues to their JSON Canvas preset numbers, blue to "7"', () => {
    const presetOf = (id: string) =>
      CANVAS_COLOR_SLOTS.find((s) => s.id === id)!.preset
    expect(presetOf('red')).toBe('1')
    expect(presetOf('orange')).toBe('2')
    expect(presetOf('yellow')).toBe('3')
    expect(presetOf('green')).toBe('4')
    expect(presetOf('cyan')).toBe('5')
    expect(presetOf('purple')).toBe('6')
    expect(presetOf('blue')).toBe('7')
  })
})

describe('paletteSlots', () => {
  it('resolves hue slots to the requested palette value; storage is the preset', () => {
    const softRed = paletteSlots('soft').find((s) => s.id === 'red')!
    const vividRed = paletteSlots('vivid').find((s) => s.id === 'red')!
    // Soft is a literal hex because its consumers do channel math on it
    // (lightenHex/darkenHex/withAlpha); vivid is the themed custom property,
    // so a swatch previews exactly what the canvas will paint.
    expect(softRed.hex).toBe('#e8b4b8')
    expect(vividRed.hex).toBe('var(--canvas-ink-red)')
    expect(softRed.storage).toBe('1')
    expect(vividRed.storage).toBe('1')
  })

  it('keeps neutral as the sentinel in both palettes', () => {
    for (const palette of ['soft', 'vivid'] as const) {
      const neutral = paletteSlots(palette).find((s) => s.id === 'neutral')!
      expect(neutral.hex).toBeNull()
      expect(neutral.storage).toBe(NEUTRAL_STORAGE)
    }
  })

  it('preserves canonical slot order', () => {
    expect(paletteSlots('vivid').map((s) => s.id)).toEqual(
      CANVAS_COLOR_SLOTS.map((s) => s.id),
    )
  })
})

describe('resolveCanvasColor', () => {
  it('resolves neutral to a contrasting fill in light vs dark mode', () => {
    const lightFill = resolveCanvasColor(NEUTRAL_STORAGE, { role: 'fill', isDark: false })
    const darkFill = resolveCanvasColor(NEUTRAL_STORAGE, { role: 'fill', isDark: true })
    expect(lightFill).not.toBe(darkFill)
  })

  it('resolves neutral ink to the themed custom property, never the fill tone', () => {
    // Ink's light/dark swap lives in surfaceTheme.css, so it resolves to the
    // same variable in both themes — what this can still pin down is that ink
    // never comes back as the fill's cream, which is what put light glyphs on
    // a light canvas before.
    const lightFill = resolveCanvasColor(NEUTRAL_STORAGE, { role: 'fill', isDark: false })
    const darkFill = resolveCanvasColor(NEUTRAL_STORAGE, { role: 'fill', isDark: true })
    for (const isDark of [false, true]) {
      const ink = resolveCanvasColor(NEUTRAL_STORAGE, { role: 'ink', isDark })
      expect(ink).toBe('var(--canvas-ink-neutral)')
      expect(ink).not.toBe(lightFill)
      expect(ink).not.toBe(darkFill)
    }
  })

  it('resolves a preset to the soft hex or the vivid custom property', () => {
    expect(resolveCanvasColor('1', { palette: 'soft' })).toBe('#e8b4b8')
    expect(resolveCanvasColor('1', { palette: 'vivid' })).toBe('var(--canvas-ink-red)')
    expect(resolveCanvasColor('7', { palette: 'soft' })).toBe('#b0c4d8')
    expect(resolveCanvasColor('7', { palette: 'vivid' })).toBe('var(--canvas-ink-blue)')
  })

  it('the same preset reads muted on one surface and punchy on another', () => {
    // Acceptance: an agent writes color "2"; a sticky shows it muted, a pen punchy.
    expect(resolveCanvasColor('2', { palette: 'soft' })).not.toBe(
      resolveCanvasColor('2', { palette: 'vivid' }),
    )
  })

  it('defaults to the soft palette when none is given', () => {
    expect(resolveCanvasColor('1')).toBe('#e8b4b8')
  })

  it('passes literal hex strings through unchanged', () => {
    expect(resolveCanvasColor('#abcdef')).toBe('#abcdef')
    expect(resolveCanvasColor('#abcdef', { palette: 'vivid' })).toBe('#abcdef')
  })
})

describe('slotForStorage', () => {
  it('matches the neutral sentinel', () => {
    expect(slotForStorage(NEUTRAL_STORAGE)).toBe('neutral')
  })

  it('matches preset numbers to their slot', () => {
    expect(slotForStorage('1')).toBe('red')
    expect(slotForStorage('3')).toBe('yellow')
    expect(slotForStorage('6')).toBe('purple')
    expect(slotForStorage('7')).toBe('blue')
  })

  it('matches a soft or vivid hex against its slot (case-insensitive)', () => {
    expect(slotForStorage('#c8b8d8')).toBe('purple')
    expect(slotForStorage('#FFE18E')).toBe('yellow')
    expect(slotForStorage('#ffe18e')).toBe('yellow')
    expect(slotForStorage('#FF1016')).toBe('red')
    expect(slotForStorage('#1084ff')).toBe('blue')
  })

  it('returns null for unknown / null / empty inputs', () => {
    expect(slotForStorage(null)).toBeNull()
    expect(slotForStorage(undefined)).toBeNull()
    expect(slotForStorage('')).toBeNull()
    expect(slotForStorage('#000000')).toBeNull()
    expect(slotForStorage('8')).toBeNull()
  })
})
