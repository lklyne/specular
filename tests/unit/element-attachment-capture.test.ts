/**
 * Capture fallback chain (ADR 0030): center hit on a meaningful element,
 * center hit on a trivial wrapper (walks up), center over nothing
 * (nearest-at-Y scan), and an empty page (body).
 *
 * There's no jsdom/happy-dom in this repo (unit tests run in plain Node —
 * see vitest.unit.config.ts), and this suite intentionally adds no new
 * dependency to get one. Instead this file hand-rolls the sliver of the DOM
 * API the capture module actually touches (elementsFromPoint, getAttribute,
 * classList, parentElement/children, getBoundingClientRect,
 * ownerDocument.querySelectorAll) as plain objects — enough to drive
 * `dom-element-utils.ts`'s real `pickContentElementAtPoint` /
 * `buildUniqueSelector` against a scripted layout, without emulating layout
 * or CSS matching in general.
 *
 * Mutation-verified by deleting the `walkUpToMeaningful` call (using the
 * raw hit unconditionally) — the "walks up" case then fails because it
 * returns the wrapper's selector instead of the meaningful ancestor's.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { captureElementAtDocumentPoint } from '../../src/preload/element-attachment-capture'

interface FakeRect {
  left: number
  top: number
  width: number
  height: number
}

class FakeElement {
  tagName: string
  parentElement: FakeElement | null = null
  children: FakeElement[] = []
  classList: string[] = []
  rect: FakeRect = { left: 0, top: 0, width: 0, height: 0 }
  shadowRoot: null = null
  ownerDocument!: FakeDocument
  private attrs = new Map<string, string>()
  position = 'static'

  constructor(tagName: string) {
    this.tagName = tagName.toUpperCase()
  }

  setAttribute(name: string, value: string): this {
    this.attrs.set(name, value)
    return this
  }

  getAttribute(name: string): string | null {
    return this.attrs.get(name) ?? null
  }

  getBoundingClientRect() {
    const { left, top, width, height } = this.rect
    return { left, top, width, height, right: left + width, bottom: top + height }
  }

  // No fixture in this file paints a Specular overlay, so nothing ever
  // matches — sufficient to exercise capture without a real CSS matcher.
  closest(_selector: string): FakeElement | null {
    return null
  }

  querySelectorAll(selector: string): FakeElement[] {
    const all = descendantsOf(this)
    if (selector === '*') return all
    if (selector.startsWith('#')) {
      const id = selector.slice(1)
      return all.filter((el) => el.getAttribute('id') === id)
    }
    return []
  }

  appendChild(child: FakeElement): FakeElement {
    child.parentElement = this
    child.ownerDocument = this.ownerDocument
    this.children.push(child)
    return child
  }
}

function descendantsOf(root: FakeElement): FakeElement[] {
  const out: FakeElement[] = []
  const walk = (el: FakeElement) => {
    for (const child of el.children) {
      out.push(child)
      walk(child)
    }
  }
  walk(root)
  return out
}

function pointInRect(rect: FakeRect, x: number, y: number): boolean {
  return x >= rect.left && x <= rect.left + rect.width && y >= rect.top && y <= rect.top + rect.height
}

class FakeDocument {
  body: FakeElement

  constructor() {
    this.body = new FakeElement('body')
    this.body.ownerDocument = this
  }

  /** Deepest-first ancestor chain at (x, y), matching real `elementsFromPoint`. */
  elementsFromPoint(x: number, y: number): FakeElement[] {
    if (!pointInRect(this.body.rect, x, y)) return []
    const chain: FakeElement[] = [this.body]
    let current = this.body
    for (;;) {
      const hit = [...current.children].reverse().find((child) => pointInRect(child.rect, x, y))
      if (!hit) break
      chain.push(hit)
      current = hit
    }
    return chain.reverse()
  }

  querySelectorAll(selector: string): FakeElement[] {
    return this.body.querySelectorAll(selector)
  }
}

let doc: FakeDocument

beforeEach(() => {
  doc = new FakeDocument()
  doc.body.rect = { left: 0, top: 0, width: 1000, height: 4000 }
  ;(globalThis as any).document = doc
  ;(globalThis as any).window = {
    scrollX: 0,
    scrollY: 0,
    innerWidth: 800,
    innerHeight: 600,
    getComputedStyle: (element: FakeElement) => ({ position: element.position }),
  }
  ;(globalThis as any).CSS = { escape: (value: string) => value }
})

afterEach(() => {
  delete (globalThis as any).document
  delete (globalThis as any).window
  delete (globalThis as any).CSS
})

function el(tag: string, attrs: Record<string, string> = {}, rect?: Partial<FakeRect>): FakeElement {
  const e = new FakeElement(tag)
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v)
  if (rect) e.rect = { left: 0, top: 0, width: 0, height: 0, ...rect }
  return e
}

describe('captureElementAtDocumentPoint', () => {
  it('returns the meaningful element directly under the point', () => {
    const hero = doc.body.appendChild(el('div', { id: 'hero' }, { left: 0, top: 100, width: 200, height: 100 }))

    const result = captureElementAtDocumentPoint(100, 150)

    expect(result).toEqual({ selector: '#hero', docX: 0, docY: 100 })
    expect(hero.getAttribute('id')).toBe('hero')
  })

  it('walks up past a trivial wrapper to the nearest meaningful ancestor', () => {
    const hero = doc.body.appendChild(el('div', { id: 'hero' }, { left: 0, top: 100, width: 200, height: 100 }))
    // A bare inline wrapper — no id/role/class, and tiny — sits at the exact
    // hit point; walking up should skip it in favor of `#hero`.
    hero.appendChild(el('span', {}, { left: 90, top: 140, width: 4, height: 4 }))

    const result = captureElementAtDocumentPoint(92, 142)

    expect(result).toEqual({ selector: '#hero', docX: 0, docY: 100 })
  })

  it('falls back to the horizontally-nearest meaningful element at that Y when the point hits nothing', () => {
    // Two content blocks at the same document Y, with a gap between them
    // that no element's rect covers.
    doc.body.appendChild(el('div', { id: 'left' }, { left: 0, top: 200, width: 100, height: 100 }))
    doc.body.appendChild(el('div', { id: 'right' }, { left: 400, top: 210, width: 100, height: 80 }))

    // docX=180 sits in the gap; #left's center (50) is closer than #right's (450).
    const result = captureElementAtDocumentPoint(180, 250)

    expect(result).toEqual({ selector: '#left', docX: 0, docY: 200 })
  })

  it('falls back to body on an empty page', () => {
    const result = captureElementAtDocumentPoint(50, 50)

    expect(result).toEqual({ selector: 'body', docX: 0, docY: 0 })
  })

  it('accounts for scroll when converting the document point to a viewport hit-test', () => {
    ;(globalThis as any).window.scrollY = 300
    const hero = doc.body.appendChild(el('div', { id: 'hero' }, { left: 0, top: 50, width: 200, height: 100 }))

    // docY=350 minus scrollY=300 lands the viewport hit-test at y=50, inside hero.
    const result = captureElementAtDocumentPoint(100, 350)

    expect(result).toEqual({ selector: '#hero', docX: 0, docY: 350 })
  })

  it('marks an element inside a sticky or fixed rail as viewport-positioned', () => {
    const rail = doc.body.appendChild(el('nav', { id: 'rail' }, { left: 0, top: 20, width: 220, height: 500 }))
    rail.position = 'sticky'
    rail.appendChild(el('div', { id: 'rail-item' }, { left: 20, top: 100, width: 160, height: 80 }))

    expect(captureElementAtDocumentPoint(50, 120)).toEqual({
      selector: '#rail-item',
      docX: 20,
      docY: 100,
      viewportPositioned: true,
    })
  })

  it('returns null when the page has no body', () => {
    ;(globalThis as any).document = { body: null }

    expect(captureElementAtDocumentPoint(0, 0)).toBeNull()
  })
})
