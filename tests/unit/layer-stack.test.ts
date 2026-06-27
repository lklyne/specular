import { describe, expect, it } from 'vitest'
import {
  LAYER_STACK,
  orderedPagesForStack,
  resolveStackOrder,
  type LayerId,
} from '../../src/main/runtime/layer-stack'

describe('LAYER_STACK', () => {
  it('is stable and bgView is first', () => {
    expect(LAYER_STACK[0]).toBe('bgView')
    expect(LAYER_STACK[LAYER_STACK.length - 1]).toBe('toolbar')
  })

  it('contains no duplicates', () => {
    expect(new Set(LAYER_STACK).size).toBe(LAYER_STACK.length)
  })

  it('resolveStackOrder drops missing refs and preserves order', () => {
    const present: Partial<Record<LayerId, unknown>> = {
      bgView: {},
      aboveView: {},
      toolbar: {},
      floatingUi: null,
      legacyInteractionOverlay: null,
      leftSidebar: null,
      devtoolsBackground: null,
      devtools: null,
      devtoolsHeader: null,
      devtoolsResizeHandle: null,
    }
    expect(resolveStackOrder(present)).toEqual(['bgView', 'aboveView', 'toolbar'])
  })

  it('resolveStackOrder with all present returns full LAYER_STACK', () => {
    const all: Partial<Record<LayerId, unknown>> = {}
    for (const id of LAYER_STACK) all[id] = {}
    expect(resolveStackOrder(all)).toEqual([...LAYER_STACK])
  })

  it('orders the focused page last among page views without mutating input order', () => {
    const pages = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]
    expect(orderedPagesForStack(pages, 'b').map((page) => page.id)).toEqual(['a', 'c', 'b'])
    expect(pages.map((page) => page.id)).toEqual(['a', 'b', 'c'])
  })

  it('leaves page order unchanged when focus presentation points at a missing page', () => {
    const pages = [{ id: 'a' }, { id: 'b' }]
    expect(orderedPagesForStack(pages, 'missing').map((page) => page.id)).toEqual(['a', 'b'])
  })
})
