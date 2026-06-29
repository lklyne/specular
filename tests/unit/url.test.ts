import { describe, it, expect } from 'vitest'
import { looksLikeUrl, resolveAddressInput } from '../../src/shared/url'

describe('looksLikeUrl', () => {
  it('accepts urls with explicit http(s) scheme', () => {
    expect(looksLikeUrl('https://example.com')).toBe(true)
    expect(looksLikeUrl('http://example.com/path?q=1')).toBe(true)
  })

  it('accepts bare host.tld inputs', () => {
    expect(looksLikeUrl('example.com')).toBe(true)
    expect(looksLikeUrl('sub.example.com/path')).toBe(true)
  })

  it('accepts localhost with optional port', () => {
    expect(looksLikeUrl('localhost')).toBe(true)
    expect(looksLikeUrl('localhost:4321')).toBe(true)
    expect(looksLikeUrl('localhost:4321/garden')).toBe(true)
  })

  it('rejects plain prose', () => {
    expect(looksLikeUrl('hello world')).toBe(false)
    expect(looksLikeUrl('hello')).toBe(false)
    expect(looksLikeUrl('')).toBe(false)
  })

  it('rejects multi-line and whitespace-bearing strings', () => {
    expect(looksLikeUrl('https://a.com\nhttps://b.com')).toBe(false)
    expect(looksLikeUrl('  https://example.com extra')).toBe(false)
  })

  it('rejects non-http schemes (avoid file://, javascript:, etc.)', () => {
    expect(looksLikeUrl('file:///tmp/foo')).toBe(false)
    expect(looksLikeUrl('javascript:alert(1)')).toBe(false)
    expect(looksLikeUrl('mailto:hi@example.com')).toBe(false)
  })

  it('trims surrounding whitespace before evaluating', () => {
    expect(looksLikeUrl('   https://example.com   ')).toBe(true)
    expect(looksLikeUrl('   example.com   ')).toBe(true)
  })
})

describe('resolveAddressInput', () => {
  it('normalizes URL-like input', () => {
    expect(resolveAddressInput('example.com/path')).toBe('https://example.com/path')
    expect(resolveAddressInput('localhost:4321')).toBe('http://localhost:4321/')
  })

  it('turns raw text into a Google search', () => {
    expect(resolveAddressInput('design systems')).toBe(
      'https://www.google.com/search?q=design%20systems',
    )
    expect(resolveAddressInput('is this thing on?')).toBe(
      'https://www.google.com/search?q=is%20this%20thing%20on%3F',
    )
    expect(resolveAddressInput('figma')).toBe('https://www.google.com/search?q=figma')
  })
})
