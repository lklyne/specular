// Guards the focus-session show/hide rule for pages (ADR 0021 + Amendment 2).
// Mutation-verified by dropping the `focus.mode !== 'fill'` clause in
// isPagePresented and confirming "eye on in fill mode still hides others"
// fails.

import { describe, expect, it } from 'vitest'
import {
  isPagePresented,
  type PagePresentationInputs,
} from '../../src/shared/page-presentation'

function inputs(focus: Partial<PagePresentationInputs['focus']>): PagePresentationInputs {
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

describe('isPagePresented', () => {
  it('shows every page when no focus session is active', () => {
    const state = inputs({})
    expect(isPagePresented('a', state)).toBe(true)
    expect(isPagePresented('b', state)).toBe(true)
  })

  it('hides other pages in a fill session with the eye off', () => {
    const state = inputs({ active: true, pageId: 'a', mode: 'fill' })
    expect(isPagePresented('a', state)).toBe(true)
    expect(isPagePresented('b', state)).toBe(false)
  })

  it('shows other pages with the eye on outside fill mode', () => {
    for (const mode of ['fit', 'device'] as const) {
      const state = inputs({ active: true, pageId: 'a', mode, annotationsVisible: true })
      expect(isPagePresented('a', state)).toBe(true)
      expect(isPagePresented('b', state)).toBe(true)
    }
  })

  it('keeps other pages hidden with the eye on in fill mode', () => {
    const state = inputs({ active: true, pageId: 'a', mode: 'fill', annotationsVisible: true })
    expect(isPagePresented('a', state)).toBe(true)
    expect(isPagePresented('b', state)).toBe(false)
  })

  it('hides every page in a file session with the eye off', () => {
    const state = inputs({ active: true, pageId: null, mode: 'fill' })
    expect(isPagePresented('a', state)).toBe(false)
    expect(isPagePresented('b', state)).toBe(false)
  })

  it('shows every page in a file session with the eye on', () => {
    const state = inputs({
      active: true,
      pageId: null,
      mode: 'fill',
      annotationsVisible: true,
    })
    expect(isPagePresented('a', state)).toBe(true)
    expect(isPagePresented('b', state)).toBe(true)
  })
})
