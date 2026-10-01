import { describe, it, expect } from 'vitest'
import {
  GENERIC_INTERACTIVE_SELECTOR,
  MAX_NAME_MATCH_CANDIDATES,
  matchElementsByName,
  matchElementsByText,
  normalizeAccessibleName,
  selectorForRole,
  type NameMatchCandidate,
} from '../../src/preload/accessible-name-matcher'

// Real data from a live Google Flights run: the row's `aria-label` is 274
// raw chars with runs of spaces before "Select flight"; agent-browser
// reports the accessibility-tree name with whitespace already collapsed. An
// exact CSS `[aria-label="..."]` selector against the raw attribute matches
// nothing — this is the normalization that lets the two sides compare equal.
const RAW_LABEL =
  'From 901 US dollars round trip total. Nonstop flight with ZIPAIR Tokyo.   ' +
  'Leaves San Francisco International Airport at 3:45 PM on Tuesday, January 12   ' +
  'and arrives at Narita International Airport at 7:55 PM on Wednesday, January 13. ' +
  'Total duration 11 hr 10 min.   Select flight'
const COLLAPSED_LABEL = RAW_LABEL.replace(/\s+/g, ' ').trim()

function candidate(overrides: Partial<NameMatchCandidate> = {}): NameMatchCandidate {
  return {
    name: null,
    text: null,
    rendered: true,
    rect: { x: 0, y: 0, width: 10, height: 10 },
    ...overrides,
  }
}

describe('normalizeAccessibleName', () => {
  it('collapses whitespace runs to a single space and trims', () => {
    expect(normalizeAccessibleName(RAW_LABEL)).toBe(COLLAPSED_LABEL)
    expect(normalizeAccessibleName('  hello   world  ')).toBe('hello world')
  })

  it('collapses NBSP and narrow-NBSP the same as ordinary whitespace', () => {
    expect(normalizeAccessibleName('Select flight')).toBe('Select flight')
    expect(normalizeAccessibleName('Select flight')).toBe('Select flight')
    expect(normalizeAccessibleName('Select    flight')).toBe('Select flight')
  })

  it('returns an empty string for null/undefined/blank input', () => {
    expect(normalizeAccessibleName(null)).toBe('')
    expect(normalizeAccessibleName(undefined)).toBe('')
    expect(normalizeAccessibleName('   ')).toBe('')
  })
})

describe('matchElementsByName', () => {
  it('matches a DOM label with irregular spacing against a collapsed query', () => {
    const row = candidate({ name: RAW_LABEL, rect: { x: 124, y: 399, width: 1022, height: 94 } })
    const matches = matchElementsByName([row], COLLAPSED_LABEL)
    expect(matches).toEqual([{ rect: row.rect }])
  })

  // The flight row's aria-label duplicated onto a zero-size element
  // elsewhere in the DOM (verified live: two elements, one rect 124,399
  // 1022x94, the other 0,0 0x0). Dropping the unrendered one is what lets
  // "exactly one match" hold for a page that genuinely has only one visible
  // row with that name.
  it('drops a zero-size duplicate so the rendered element counts as the unique match', () => {
    const rendered = candidate({ name: 'Select flight', rect: { x: 124, y: 399, width: 1022, height: 94 } })
    const zeroSize = candidate({ name: 'Select flight', rendered: false, rect: { x: 0, y: 0, width: 0, height: 0 } })
    const matches = matchElementsByName([zeroSize, rendered], 'Select flight')
    expect(matches).toEqual([{ rect: rendered.rect }])
  })

  it('returns every rendered match when a name is genuinely ambiguous', () => {
    const first = candidate({ name: 'Docs', rect: { x: 0, y: 0, width: 10, height: 10 } })
    const second = candidate({ name: 'Docs', rect: { x: 0, y: 50, width: 10, height: 10 } })
    const matches = matchElementsByName([first, second], 'Docs')
    // The matcher only finds elements with this name — refusing to travel on
    // an ambiguous match is a caller decision (findPresenceTarget), not this
    // function's.
    expect(matches).toEqual([{ rect: first.rect }, { rect: second.rect }])
  })

  it('returns nothing for a blank query', () => {
    expect(matchElementsByName([candidate({ name: 'Anything' })], '   ')).toEqual([])
  })

  it('only examines the first MAX_NAME_MATCH_CANDIDATES candidates', () => {
    const padding = Array.from({ length: MAX_NAME_MATCH_CANDIDATES }, () => candidate({ name: 'filler' }))
    const target = candidate({ name: 'Target', rect: { x: 1, y: 2, width: 3, height: 4 } })

    // Beyond the cap: never examined.
    expect(matchElementsByName([...padding, target], 'Target')).toEqual([])

    // Within the cap: found.
    expect(matchElementsByName([target, ...padding], 'Target')).toEqual([{ rect: target.rect }])
  })
})

describe('matchElementsByText', () => {
  it('matches on visible text rather than accessible name', () => {
    const el = candidate({ name: 'button', text: 'Sign in now', rect: { x: 1, y: 1, width: 1, height: 1 } })
    expect(matchElementsByText([el], 'Sign in now')).toEqual([{ rect: el.rect }])
    expect(matchElementsByText([el], 'button')).toEqual([])
  })
})

describe('selectorForRole', () => {
  it('maps a known role to its selector', () => {
    expect(selectorForRole('link')).toBe('a[href],[role="link"]')
    expect(selectorForRole('button')).toContain('button')
  })

  it('falls back to the generic interactive selector for an absent or unknown role', () => {
    expect(selectorForRole(null)).toBe(GENERIC_INTERACTIVE_SELECTOR)
    expect(selectorForRole(undefined)).toBe(GENERIC_INTERACTIVE_SELECTOR)
    expect(selectorForRole('some-made-up-role')).toBe(GENERIC_INTERACTIVE_SELECTOR)
  })
})
