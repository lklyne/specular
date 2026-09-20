/**
 * Pure matching logic behind the page-side accessible-name lookup (issue
 * #319 follow-up): given a page's rendered elements as plain descriptors,
 * find the ones whose computed name/text exactly equals a query value once
 * both sides are whitespace-normalized. Kept DOM-free so it is unit-testable
 * with plain data — `dom-element-utils.ts` gathers the descriptors from the
 * live document and calls in here; it owns no matching logic of its own.
 */

// Collapses whitespace runs, including the two non-breaking-space variants
// browsers substitute at word-wrap boundaries, to a single space. This is
// what lets a hand-authored `aria-label` full of double/triple spaces match
// agent-browser's collapsed accessibility-tree name for the same element —
// verified against a live Google Flights row: a 274-char `aria-label` with
// runs of spaces before "Select flight" collapses to the same string
// agent-browser reported.
const WHITESPACE_RUN = /[\s\u00A0\u202F]+/g

export function normalizeAccessibleName(value: string | null | undefined): string {
  if (!value) return ''
  return value.replace(WHITESPACE_RUN, ' ').trim()
}

/** ARIA roles agent-browser's snapshot vocabulary reports, mapped to a CSS
 *  selector that finds the native + role-carrying elements for that role.
 *  Deliberately narrow — a role we don't recognize falls back to
 *  `GENERIC_INTERACTIVE_SELECTOR` rather than guessing a selector for it. */
export const ROLE_SELECTORS: Record<string, string> = {
  link: 'a[href],[role="link"]',
  button: 'button,[role="button"],input[type="button"],input[type="submit"],input[type="reset"]',
  textbox: 'input:not([type="hidden"]),textarea,[role="textbox"],[contenteditable]',
  combobox: 'select,[role="combobox"]',
  checkbox: 'input[type="checkbox"],[role="checkbox"]',
  radio: 'input[type="radio"],[role="radio"]',
  tab: '[role="tab"]',
  option: 'option,[role="option"]',
  menuitem: '[role="menuitem"]',
  heading: 'h1,h2,h3,h4,h5,h6,[role="heading"]',
}

/** Used when the role is absent or unrecognized — the same interactive
 *  vocabulary `isInteractiveForSnapshot` treats as interactive, so the
 *  fallback candidate set lines up with what agent-browser itself would
 *  call an element worth naming. */
export const GENERIC_INTERACTIVE_SELECTOR =
  'a[href],button,input:not([type="hidden"]),select,textarea,summary,[role],[tabindex]'

export function selectorForRole(role: string | null | undefined): string {
  if (!role) return GENERIC_INTERACTIVE_SELECTOR
  return ROLE_SELECTORS[role] ?? GENERIC_INTERACTIVE_SELECTOR
}

export interface NameMatchRect {
  x: number
  y: number
  width: number
  height: number
}

export interface NameMatchCandidate {
  name: string | null
  text: string | null
  rendered: boolean
  rect: NameMatchRect
}

export interface NameMatch {
  rect: NameMatchRect
}

/** Caps how many candidates a lookup examines — a pathological page can't
 *  stall the renderer computing names for thousands of elements. Shared with
 *  the DOM-side gatherer, which slices its `querySelectorAll` result to the
 *  same bound before computing anything. */
export const MAX_NAME_MATCH_CANDIDATES = 2000

/** At most this many matches are returned. The only decision callers make on
 *  the result is "exactly one" vs. "not exactly one" (findPresenceTarget
 *  refuses to travel on anything but a unique match), so describing more
 *  than a couple of duplicates in full is wasted payload. */
const MAX_NAME_MATCHES = 5

function matchByField(
  candidates: NameMatchCandidate[],
  field: 'name' | 'text',
  queryValue: string,
): NameMatch[] {
  const wanted = normalizeAccessibleName(queryValue)
  if (!wanted) return []
  const matches: NameMatch[] = []
  for (const candidate of candidates.slice(0, MAX_NAME_MATCH_CANDIDATES)) {
    if (!candidate.rendered) continue
    if (normalizeAccessibleName(candidate[field]) !== wanted) continue
    matches.push({ rect: candidate.rect })
    if (matches.length >= MAX_NAME_MATCHES) break
  }
  return matches
}

/** Rendered candidates whose computed accessible name exactly equals
 *  `queryName` after whitespace normalization. */
export function matchElementsByName(
  candidates: NameMatchCandidate[],
  queryName: string,
): NameMatch[] {
  return matchByField(candidates, 'name', queryName)
}

/** Same contract as `matchElementsByName`, over visible text instead of the
 *  accessible name — the fallback for `text=` locators. */
export function matchElementsByText(
  candidates: NameMatchCandidate[],
  queryText: string,
): NameMatch[] {
  return matchByField(candidates, 'text', queryText)
}
