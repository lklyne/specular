// Guards PageTextureSurface's draw order: non-presented pages are excluded
// and the focused page always sorts last, so its texture paints over its
// neighbors. Mutation-verified by dropping the "move focused page last" step
// in orderTexturePages and confirming "moves the focused page to the end"
// fails.

import { describe, expect, it } from 'vitest'
import { orderTexturePages } from '../../src/renderer/canvas-bg/texturePageOrder'
import type { PagePresentationInputs } from '../../src/shared/page-presentation'
import type { ProjectedPageEntity } from '../../src/shared/scene-projection'

function page(id: string): ProjectedPageEntity {
  return { id } as unknown as ProjectedPageEntity
}

function presentation(focus: Partial<PagePresentationInputs['focus']>): PagePresentationInputs {
  return {
    focus: {
      pageId: null,
      mode: null,
      annotationsVisible: false,
      active: false,
      ...focus,
    },
  }
}

describe('orderTexturePages', () => {
  it('keeps scene order when no focus session is active', () => {
    const pages = [page('a'), page('b'), page('c')]
    const result = orderTexturePages(pages, presentation({}))
    expect(result.map((p) => p.id)).toEqual(['a', 'b', 'c'])
  })

  it('excludes pages a focus session hides', () => {
    const pages = [page('a'), page('b'), page('c')]
    const result = orderTexturePages(pages, presentation({ active: true, pageId: 'b', mode: 'fill' }))
    expect(result.map((p) => p.id)).toEqual(['b'])
  })

  it('moves the focused page to the end so it draws on top', () => {
    const pages = [page('a'), page('b'), page('c')]
    const result = orderTexturePages(
      pages,
      presentation({ active: true, pageId: 'b', mode: 'fit', annotationsVisible: true }),
    )
    expect(result.map((p) => p.id)).toEqual(['a', 'c', 'b'])
  })

  it('is a no-op when the focused page id is not in the presented list', () => {
    const pages = [page('a'), page('c')]
    const result = orderTexturePages(
      pages,
      presentation({ active: true, pageId: 'missing', mode: 'fit', annotationsVisible: true }),
    )
    expect(result.map((p) => p.id)).toEqual(['a', 'c'])
  })
})
