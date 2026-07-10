import { describe, it, expect } from 'vitest'
import { parseTargetQuery } from '../../src/main/mcp-browse'

// D11 (issue #318 follow-up): re-resolving targets (CSS selector, text=
// locator, find role/testid) need to flow through /session/presence/intent
// as a targetQuery so the intent handler can pre-resolve them via
// findPresenceTarget. This is the pure extraction at the front of that path
// — everything downstream (the intent payload, the async resolution in
// session.ts) is plumbing around what this function decides to extract.
describe('parseTargetQuery', () => {
  it('extracts a CSS selector target for click', () => {
    expect(parseTargetQuery('click "#submit"')).toEqual({
      selector: '#submit', text: null, role: null, name: null,
    })
  })

  it('extracts a CSS selector with combinators for select', () => {
    expect(parseTargetQuery('select "div.menu > option" "Second"')).toEqual({
      selector: 'div.menu > option', text: null, role: null, name: null,
    })
  })

  it('extracts a text= locator, stripping the prefix', () => {
    expect(parseTargetQuery('click "text=Submit"')).toEqual({
      selector: null, text: 'Submit', role: null, name: null,
    })
  })

  it('extracts a text= locator with embedded spaces for fill', () => {
    expect(parseTargetQuery('fill "text=Sign in" "hello"')).toEqual({
      selector: null, text: 'Sign in', role: null, name: null,
    })
  })

  it('extracts role + --name from the find-form', () => {
    expect(parseTargetQuery('find role button click --name "Submit"')).toEqual({
      selector: null, text: null, role: 'button', name: 'Submit',
    })
  })

  it('extracts find-form role without --name', () => {
    expect(parseTargetQuery('find role link click')).toEqual({
      selector: null, text: null, role: 'link', name: null,
    })
  })

  it('maps find-form testid to a data-testid attribute selector', () => {
    expect(parseTargetQuery('find testid submit-btn click --name "Go"')).toEqual({
      selector: '[data-testid="submit-btn"]', text: null, role: null, name: 'Go',
    })
  })

  it('returns null for an @eN ref target — refs are already opaque to specular', () => {
    expect(parseTargetQuery('click @e5')).toBeNull()
    expect(parseTargetQuery('fill @e12 "hello"')).toBeNull()
  })

  it('returns null for non-target-taking verbs', () => {
    expect(parseTargetQuery('snapshot -i')).toBeNull()
    expect(parseTargetQuery('wait --load networkidle')).toBeNull()
    expect(parseTargetQuery('scroll down')).toBeNull()
  })

  it('returns null for an empty command', () => {
    expect(parseTargetQuery('')).toBeNull()
  })

  it('returns null for an unrecognized find locator-kind', () => {
    expect(parseTargetQuery('find bogus-kind value click')).toBeNull()
  })

  it('skips a leading --cdp flag before the verb', () => {
    expect(parseTargetQuery('--cdp ws://localhost:9222 click "#go"')).toEqual({
      selector: '#go', text: null, role: null, name: null,
    })
  })
})
