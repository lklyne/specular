import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  __resetRegistryForTests,
  getRendererTagFor,
  listRegisteredRenderers,
  pickRenderer,
  registerEntityRenderer,
  unregisterEntityRenderer,
  type EntityRendererClaim,
} from '../../src/main/plugins/registry'
import type { PersistedFileEntity } from '../../src/shared/types'

function fileEntity(overrides: Partial<PersistedFileEntity> = {}): PersistedFileEntity {
  return {
    kind: 'file',
    id: 'f1',
    file: 'src/Button.tsx',
    canvasX: 0,
    canvasY: 0,
    width: 320,
    height: 240,
    ...overrides,
  }
}

afterEach(() => {
  __resetRegistryForTests()
})

describe('entity-renderer registry', () => {
  it('returns null when no renderer claims the entity', () => {
    expect(pickRenderer(fileEntity())).toBeNull()
    expect(getRendererTagFor(fileEntity())).toBeNull()
  })

  it('first registered claim that matches wins', () => {
    const first: EntityRendererClaim = {
      id: 'first',
      kind: 'inline',
      rendererTag: 'markdown',
      editable: true,
      claims: (e) => /\.md$/i.test(e.file),
    }
    const second: EntityRendererClaim = {
      id: 'second',
      kind: 'inline',
      rendererTag: 'markdown',
      editable: true,
      claims: (e) => /\.md$/i.test(e.file),
    }
    registerEntityRenderer(first)
    registerEntityRenderer(second)
    expect(pickRenderer(fileEntity({ file: 'notes.md' }))?.id).toBe('first')
  })

  it('plugins surface their rendererTag via getRendererTagFor', () => {
    registerEntityRenderer({
      id: 'md',
      kind: 'inline',
      rendererTag: 'markdown',
      editable: true,
      claims: (e) => /\.md$/i.test(e.file),
    })
    expect(getRendererTagFor(fileEntity({ file: 'a.md' }))).toBe('markdown')
  })

  it('wcv-page plugins surface their tag too (for placeholder rendering)', () => {
    registerEntityRenderer({
      id: 'comp',
      kind: 'wcv-page',
      rendererTag: 'component',
      editable: false,
      claims: (e) => /\.tsx$/i.test(e.file),
      resolveUrl: () => 'http://example.test',
    })
    const entity = fileEntity({ file: 'Button.tsx' })
    expect(pickRenderer(entity)?.id).toBe('comp')
    expect(getRendererTagFor(entity)).toBe('component')
  })

  it('unregister removes the claim', () => {
    registerEntityRenderer({
      id: 'tmp',
      kind: 'inline',
      rendererTag: 'markdown',
      editable: true,
      claims: () => true,
    })
    expect(unregisterEntityRenderer('tmp')).toBe(true)
    expect(pickRenderer(fileEntity())).toBeNull()
    expect(unregisterEntityRenderer('tmp')).toBe(false)
  })

  it('throws when registering a duplicate id', () => {
    const claim: EntityRendererClaim = {
      id: 'dup',
      kind: 'inline',
      rendererTag: 'markdown',
      editable: true,
      claims: () => true,
    }
    registerEntityRenderer(claim)
    expect(() => registerEntityRenderer(claim)).toThrow(/already registered/)
  })

  it('listRegisteredRenderers preserves registration order', () => {
    const ids = ['a', 'b', 'c']
    for (const id of ids) {
      registerEntityRenderer({
        id,
        kind: 'inline',
        rendererTag: 'markdown',
        editable: true,
        claims: () => false,
      })
    }
    expect(listRegisteredRenderers().map((c) => c.id)).toEqual(ids)
  })

  it('higher-priority claims are tested before lower-priority ones', () => {
    registerEntityRenderer({
      id: 'low',
      kind: 'inline',
      rendererTag: 'markdown',
      editable: true,
      claims: (e) => /\.json$/i.test(e.file),
    })
    registerEntityRenderer({
      id: 'high',
      kind: 'inline',
      rendererTag: 'markdown',
      priority: 10,
      editable: true,
      claims: (e) => /\.special\.json$/i.test(e.file),
    })
    expect(pickRenderer(fileEntity({ file: 'foo.special.json' }))?.id).toBe('high')
    expect(pickRenderer(fileEntity({ file: 'foo.json' }))?.id).toBe('low')
  })

  it('same-priority claims fall back to registration order', () => {
    registerEntityRenderer({
      id: 'first',
      kind: 'inline',
      rendererTag: 'markdown',
      editable: true,
      claims: (e) => /\.txt$/i.test(e.file),
    })
    registerEntityRenderer({
      id: 'second',
      kind: 'inline',
      rendererTag: 'image',
      editable: false,
      claims: (e) => /\.txt$/i.test(e.file),
    })
    expect(pickRenderer(fileEntity({ file: 'a.txt' }))?.id).toBe('first')
  })

  it('a throwing claims() predicate does not blank out later plugins', () => {
    const errSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    registerEntityRenderer({
      id: 'broken',
      kind: 'inline',
      rendererTag: 'markdown',
      editable: true,
      claims: () => {
        throw new Error('boom')
      },
    })
    registerEntityRenderer({
      id: 'healthy',
      kind: 'inline',
      rendererTag: 'image',
      editable: false,
      claims: (e) => /\.png$/i.test(e.file),
    })
    const picked = pickRenderer(fileEntity({ file: 'photo.png' }))
    expect(picked?.id).toBe('healthy')
    expect(errSpy).toHaveBeenCalledWith(
      expect.stringContaining('"broken"'),
      expect.any(Error),
    )
    errSpy.mockRestore()
  })
})

describe('built-in component-render plugin', () => {
  it('claims .tsx and .jsx files', async () => {
    const { componentRenderPlugin } = await import(
      '../../src/main/plugins/builtin/component-render'
    )
    expect(componentRenderPlugin.kind).toBe('wcv-page')
    expect(componentRenderPlugin.claims(fileEntity({ file: 'Foo.tsx' }))).toBe(true)
    expect(componentRenderPlugin.claims(fileEntity({ file: 'Foo.jsx' }))).toBe(true)
    expect(componentRenderPlugin.claims(fileEntity({ file: 'Foo.ts' }))).toBe(false)
    expect(componentRenderPlugin.claims(fileEntity({ file: 'a.md' }))).toBe(false)
  })
})
