import { describe, expect, it } from 'vitest'
import { decodePercentEncoded, dragMayLoad } from '../../src/main/runtime/page-drag-policy'

describe('dragMayLoad', () => {
  const remote = 'https://example.com/article'
  const local = 'file:///Users/someone/space/notes.html'

  it('lets any page drag out http(s) content', () => {
    expect(dragMayLoad('https://cdn.example.com/cat.png', remote)).toBe(true)
    expect(dragMayLoad('http://cdn.example.com/cat.png', local)).toBe(true)
  })

  it('refuses a local file dragged out of a remote page', () => {
    expect(dragMayLoad('file:///Users/someone/.ssh/id_rsa', remote)).toBe(false)
  })

  it('allows a local file dragged out of a local page', () => {
    expect(dragMayLoad('file:///Users/someone/space/cat.png', local)).toBe(true)
  })

  it('refuses a local file when the source page is unknown', () => {
    expect(dragMayLoad('file:///Users/someone/.ssh/id_rsa', undefined)).toBe(false)
  })

  it('refuses schemes that are neither http(s) nor file', () => {
    expect(dragMayLoad('chrome://settings', local)).toBe(false)
    expect(dragMayLoad('javascript:alert(1)', remote)).toBe(false)
    expect(dragMayLoad('not a url', remote)).toBe(false)
  })
})

describe('decodePercentEncoded', () => {
  it('decodes percent escapes', () => {
    expect(decodePercentEncoded('a%20b').toString('utf-8')).toBe('a b')
  })

  it('passes through a percent that begins no escape', () => {
    // An inline SVG data URL that decodeURIComponent throws on.
    const svg = '<svg width="100%" height="50%"><rect/></svg>'
    expect(decodePercentEncoded(svg).toString('utf-8')).toBe(svg)
  })

  it('keeps multi-byte escapes intact', () => {
    expect(decodePercentEncoded('%E2%9C%93').toString('utf-8')).toBe('✓')
  })

  it('handles a trailing percent', () => {
    expect(decodePercentEncoded('width:100%').toString('utf-8')).toBe('width:100%')
  })
})
