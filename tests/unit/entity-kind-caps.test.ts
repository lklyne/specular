import { describe, expect, it } from 'vitest'
import { ENTITY_KIND_CAPS } from '../../src/shared/entity-kind-caps'
import type { CanvasEntityKind } from '../../src/shared/types'

describe('ENTITY_KIND_CAPS', () => {
  it('has a row for every canvas entity kind', () => {
    const kinds: CanvasEntityKind[] = [
      'page',
      'text',
      'file',
      'group',
      'edge',
      'drawing',
      'shape',
    ]
    for (const kind of kinds) {
      expect(ENTITY_KIND_CAPS[kind]).toBeDefined()
    }
    expect(Object.keys(ENTITY_KIND_CAPS).sort()).toEqual([...kinds].sort())
  })

  it('marks chrome only for pages and files', () => {
    expect(ENTITY_KIND_CAPS.page.hasChrome).toBe(true)
    expect(ENTITY_KIND_CAPS.file.hasChrome).toBe(true)
    expect(ENTITY_KIND_CAPS.text.hasChrome).toBe(false)
    expect(ENTITY_KIND_CAPS.shape.hasChrome).toBe(false)
    expect(ENTITY_KIND_CAPS.group.hasChrome).toBe(false)
    expect(ENTITY_KIND_CAPS.drawing.hasChrome).toBe(false)
  })

  it('withholds anchors from drawings', () => {
    expect(ENTITY_KIND_CAPS.drawing.hasAnchors).toBe(false)
    expect(ENTITY_KIND_CAPS.page.hasAnchors).toBe(true)
    expect(ENTITY_KIND_CAPS.text.hasAnchors).toBe(true)
    expect(ENTITY_KIND_CAPS.file.hasAnchors).toBe(true)
    expect(ENTITY_KIND_CAPS.group.hasAnchors).toBe(true)
    expect(ENTITY_KIND_CAPS.shape.hasAnchors).toBe(true)
  })

  it('carries per-kind resize config', () => {
    expect(ENTITY_KIND_CAPS.page.minSize).toEqual({ width: 320, height: 200 })
    expect(ENTITY_KIND_CAPS.shape.minSize).toEqual({ width: 24, height: 24 })
    expect(ENTITY_KIND_CAPS.shape.aspectMode).toBe('shift-locks')
    expect(ENTITY_KIND_CAPS.page.aspectMode).toBe('off')
  })
})
