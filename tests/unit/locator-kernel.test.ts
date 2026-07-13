import { describe, expect, it } from 'vitest'
import {
  LOCATOR_CONFIDENCE_FLOOR,
  LOCATOR_RUNNER_UP_MARGIN,
  dispatchPointForCandidate,
  resolveLocator,
  type LocatorBundle,
  type LocatorCandidate,
} from '../../src/shared/locator-kernel'

// Mutation check: weaken the confidence policy in `resolveLocator` (e.g. drop
// the identity-key short-circuit, or lower the floor/margin so ambiguous
// same-text buttons resolve confident) and these fail.

function bundle(overrides: Partial<LocatorBundle>): LocatorBundle {
  return {
    tag: 'div',
    elementPath: '',
    fullPath: '',
    offsetX: 0.5,
    offsetY: 0.5,
    ...overrides,
  }
}

function candidate(overrides: Partial<LocatorCandidate>): LocatorCandidate {
  return {
    id: null,
    testId: null,
    role: null,
    name: null,
    text: null,
    tag: null,
    elementPath: null,
    fullPath: null,
    interactive: false,
    rect: { x: 0, y: 0, width: 10, height: 10 },
    ...overrides,
  }
}

// Bounds far from the origin so the top-left proximity tiebreak contributes 0,
// keeping scores exact for the margin/floor boundary cases below.
const FAR_RECT = { x: 10_000, y: 10_000, width: 10, height: 10 }

describe('resolveLocator identity keys', () => {
  it('exact-id match wins', () => {
    const result = resolveLocator(bundle({ id: 'submit-btn' }), [
      candidate({ id: 'submit-btn', rect: { x: 10, y: 20, width: 40, height: 20 } }),
      candidate({ id: 'cancel-btn' }),
    ])
    expect(result.kind).toBe('confident')
    if (result.kind === 'confident') {
      expect(result.candidate.id).toBe('submit-btn')
      // offset 0.5 into the matched rect.
      expect(result.point).toEqual({ x: 30, y: 30 })
    }
  })

  it('unique data-testid wins even when text is shared', () => {
    const result = resolveLocator(bundle({ testId: 'hero-cta', text: 'Buy now' }), [
      candidate({ testId: 'hero-cta', text: 'Buy now', interactive: true }),
      candidate({ testId: 'footer-cta', text: 'Buy now', interactive: true }),
    ])
    expect(result.kind).toBe('confident')
    if (result.kind === 'confident') expect(result.candidate.testId).toBe('hero-cta')
  })

  it('identity match beats structural drift', () => {
    // The bundle's id and paths describe the old DOM; only the id still exists.
    // A decoy that keeps the old paths/name scores higher structurally, but the
    // identity key must win.
    const drifted = candidate({
      id: 'go',
      name: 'Go',
      elementPath: 'div > section > button',
    })
    const decoy = candidate({
      name: 'Go',
      text: 'Go',
      elementPath: 'main > form > button#go',
      interactive: true,
    })
    const result = resolveLocator(
      bundle({ id: 'go', name: 'Go', elementPath: 'main > form > button#go' }),
      [decoy, drifted],
    )
    expect(result.kind).toBe('confident')
    if (result.kind === 'confident') expect(result.candidate.id).toBe('go')
  })

  it('restructured DOM (paths differ, identity holds) resolves confident', () => {
    const result = resolveLocator(
      bundle({
        id: 'nav-home',
        elementPath: 'header > nav > a',
        fullPath: 'body > header > nav > a',
      }),
      [
        candidate({
          id: 'nav-home',
          elementPath: 'div.wrapper > ul > li > a',
          fullPath: 'body > div > ul > li > a',
          rect: { x: 0, y: 0, width: 50, height: 20 },
        }),
      ],
    )
    expect(result.kind).toBe('confident')
  })

  it('duplicated identity key refuses rather than guessing', () => {
    const result = resolveLocator(bundle({ id: 'dup' }), [
      candidate({ id: 'dup' }),
      candidate({ id: 'dup' }),
    ])
    expect(result.kind).toBe('ambiguous')
  })
})

describe('resolveLocator structural scoring', () => {
  it('two same-text buttons are ambiguous', () => {
    const btn = { text: 'Add to cart', interactive: true, tag: 'button' }
    const result = resolveLocator(bundle({ text: 'Add to cart', tag: 'button' }), [
      candidate(btn),
      candidate(btn),
    ])
    expect(result.kind).toBe('ambiguous')
  })

  it('role + tag agreement disambiguates two same-text controls', () => {
    // Same text (+320) and interactive (+50) on both, far so proximity is 0 —
    // a tie without role/tag. The bundle is a <button role="button">; the
    // matching control agrees (+80 role, +40 tag) while the link twin does not,
    // clearing the runner-up margin so the button resolves confidently.
    const result = resolveLocator(
      bundle({ text: 'Add to cart', role: 'button', tag: 'button' }),
      [
        candidate({ text: 'Add to cart', interactive: true, role: 'button', tag: 'button', rect: FAR_RECT }),
        candidate({ text: 'Add to cart', interactive: true, role: 'link', tag: 'a', rect: FAR_RECT }),
      ],
    )
    expect(result.kind).toBe('confident')
    if (result.kind === 'confident') expect(result.candidate.role).toBe('button')
  })

  it('an absent element resolves to none', () => {
    const result = resolveLocator(bundle({ id: 'missing', name: 'Ghost button' }), [
      candidate({ id: 'other', name: 'Something else', text: 'unrelated' }),
    ])
    expect(result.kind).toBe('none')
  })
})

describe('resolveLocator confidence boundaries', () => {
  it('is confident when the top score clears the runner-up by exactly the margin', () => {
    // Exact name (+400) vs includes-name (+280); both interactive (+50) and far
    // (proximity 0) → scores 450 vs 330, a gap of exactly LOCATOR_RUNNER_UP_MARGIN.
    expect(LOCATOR_RUNNER_UP_MARGIN).toBe(120)
    const result = resolveLocator(bundle({ name: 'Save' }), [
      candidate({ name: 'Save', interactive: true, rect: FAR_RECT }),
      candidate({ name: 'Save changes', interactive: true, rect: FAR_RECT }),
    ])
    expect(result.kind).toBe('confident')
    if (result.kind === 'confident') expect(result.candidate.name).toBe('Save')
  })

  it('is ambiguous when two candidates tie under the margin', () => {
    const result = resolveLocator(bundle({ name: 'Save' }), [
      candidate({ name: 'Save', interactive: true, rect: FAR_RECT }),
      candidate({ name: 'Save', interactive: true, rect: FAR_RECT }),
    ])
    expect(result.kind).toBe('ambiguous')
  })

  it('refuses a lone match below the confidence floor', () => {
    // text-includes (+200), non-interactive, far → 200 < LOCATOR_CONFIDENCE_FLOOR.
    expect(LOCATOR_CONFIDENCE_FLOOR).toBe(300)
    const result = resolveLocator(bundle({ text: 'buy' }), [
      candidate({ text: 'buy now and save big', interactive: false, rect: FAR_RECT }),
    ])
    expect(result.kind).toBe('none')
  })

  it('accepts a lone match above the confidence floor', () => {
    // text-exact (+320) + interactive (+50) = 370 ≥ floor, no runner-up.
    const result = resolveLocator(bundle({ text: 'buy' }), [
      candidate({ text: 'buy', interactive: true, rect: FAR_RECT }),
    ])
    expect(result.kind).toBe('confident')
  })
})

describe('dispatchPointForCandidate', () => {
  const rect = { x: 100, y: 200, width: 40, height: 20 }

  it('maps an offset fraction to a point inside the rect', () => {
    expect(dispatchPointForCandidate(rect, 0.5, 0.5)).toEqual({ x: 120, y: 210 })
    expect(dispatchPointForCandidate(rect, 0, 0)).toEqual({ x: 100, y: 200 })
    expect(dispatchPointForCandidate(rect, 1, 1)).toEqual({ x: 140, y: 220 })
  })

  it('clamps out-of-range fractions into the rect', () => {
    expect(dispatchPointForCandidate(rect, 1.5, -0.3)).toEqual({ x: 140, y: 200 })
  })

  it('round-trips through a confident resolution', () => {
    const target = { x: 100, y: 200, width: 40, height: 20 }
    const result = resolveLocator(
      bundle({ id: 'target', offsetX: 0.25, offsetY: 0.75 }),
      [candidate({ id: 'target', rect: target })],
    )
    expect(result.kind).toBe('confident')
    if (result.kind === 'confident') {
      expect(result.point).toEqual(dispatchPointForCandidate(target, 0.25, 0.75))
      expect(result.point).toEqual({ x: 110, y: 215 })
    }
  })
})
