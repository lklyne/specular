import { describe, expect, it } from 'vitest'
import {
  DEFAULT_TOOL_DEFAULTS,
  normalizeToolDefaults,
} from '../../src/shared/tool-defaults'

describe('tool-defaults: normalizeToolDefaults', () => {
  it('returns full defaults when input is undefined', () => {
    const out = normalizeToolDefaults(undefined)
    expect(out).toEqual(DEFAULT_TOOL_DEFAULTS)
  })

  it('returns full defaults when input is malformed', () => {
    expect(normalizeToolDefaults(null)).toEqual(DEFAULT_TOOL_DEFAULTS)
    expect(normalizeToolDefaults('garbage')).toEqual(DEFAULT_TOOL_DEFAULTS)
    expect(normalizeToolDefaults(42)).toEqual(DEFAULT_TOOL_DEFAULTS)
  })

  it('round-trips a complete persisted blob', () => {
    const persisted = {
      'add-text': { color: '5', textSize: 32 },
      'add-sticky': { color: '1', textSize: 18 },
      'add-shape': {
        shapeKind: 'ellipse' as const,
        color: '#abcdef',
        strokeWidth: 4,
        textSize: 56,
      },
      draw: { brushType: 'highlight' as const, color: '#111111', strokeWidth: 6 },
    }
    expect(normalizeToolDefaults(persisted)).toEqual(persisted)
  })

  it('defaults add-shape.textSize when absent (legacy preferences)', () => {
    const out = normalizeToolDefaults({
      'add-shape': { shapeKind: 'rectangle' as const, color: '1', strokeWidth: 2 },
    })
    expect(out['add-shape'].textSize).toBe(DEFAULT_TOOL_DEFAULTS['add-shape'].textSize)
  })

  it('accepts a custom add-shape.textSize', () => {
    const out = normalizeToolDefaults({
      'add-shape': {
        shapeKind: 'rectangle' as const,
        color: '1',
        strokeWidth: 2,
        textSize: 96,
      },
    })
    expect(out['add-shape'].textSize).toBe(96)
  })

  it('drops a legacy textKind key (creation kind is now tool-selected)', () => {
    const out = normalizeToolDefaults({
      'add-text': { color: '5', textSize: 32, textKind: 'long' },
    })
    expect(out['add-text']).toEqual({ color: '5', textSize: 32 })
  })

  it('fills gaps with defaults when only one scope is persisted', () => {
    const partial = { draw: { brushType: 'highlight' as const, color: '#aaa', strokeWidth: 9 } }
    const out = normalizeToolDefaults(partial)
    expect(out.draw).toEqual(partial.draw)
    expect(out['add-text']).toEqual(DEFAULT_TOOL_DEFAULTS['add-text'])
    expect(out['add-sticky']).toEqual(DEFAULT_TOOL_DEFAULTS['add-sticky'])
    expect(out['add-shape']).toEqual(DEFAULT_TOOL_DEFAULTS['add-shape'])
  })

  it('rejects unknown shapeKind / brushType, keeps the default', () => {
    const out = normalizeToolDefaults({
      'add-shape': { shapeKind: 'octagon', color: '#fff', strokeWidth: 1 },
      draw: { brushType: 'spray', color: '#fff', strokeWidth: 1 },
    })
    expect(out['add-shape'].shapeKind).toBe(DEFAULT_TOOL_DEFAULTS['add-shape'].shapeKind)
    expect(out.draw.brushType).toBe(DEFAULT_TOOL_DEFAULTS.draw.brushType)
    // But valid sibling fields still come through:
    expect(out['add-shape'].color).toBe('#fff')
    expect(out['add-shape'].strokeWidth).toBe(1)
  })

  it('accepts text color null as a valid override (inherit)', () => {
    const out = normalizeToolDefaults({
      'add-text': { color: null, textSize: 32 },
      'add-sticky': { color: '2', textSize: 56 },
    })
    expect(out['add-text'].color).toBe(null)
    expect(out['add-text'].textSize).toBe(32)
    expect(out['add-sticky'].color).toBe('2')
    expect(out['add-sticky'].textSize).toBe(56)
  })

  it('migrates legacy text default color keys', () => {
    const out = normalizeToolDefaults({
      'add-text': { 'sticky.color': '2', 'plain.color': null },
    })
    expect(out['add-text'].color).toBe(null)
    expect(out['add-sticky'].color).toBe('2')
  })

  it('isolates per-tool keys: changing draw color does not affect shape color', () => {
    const persisted = {
      'add-shape': { ...DEFAULT_TOOL_DEFAULTS['add-shape'], color: '#aabbcc' },
      draw: { ...DEFAULT_TOOL_DEFAULTS.draw, color: '#112233' },
    }
    const out = normalizeToolDefaults(persisted)
    expect(out['add-shape'].color).toBe('#aabbcc')
    expect(out.draw.color).toBe('#112233')
  })
})
