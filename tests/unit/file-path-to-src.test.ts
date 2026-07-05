import { describe, it, expect } from 'vitest'
import {
  filePathToSrc,
  filePathToSrcVersioned,
} from '../../src/renderer/canvas-bg/entity-renderers/filePathToSrc'

describe('filePathToSrc', () => {
  it('wraps a bare path in the local-file:// scheme', () => {
    expect(filePathToSrc('/tmp/x.html')).toBe('local-file:///tmp/x.html')
  })

  it('leaves already-schemed urls untouched', () => {
    expect(filePathToSrc('https://example.com')).toBe('https://example.com')
    expect(filePathToSrc('local-file:///a')).toBe('local-file:///a')
  })
})

describe('filePathToSrcVersioned', () => {
  it('omits the cache-buster at version 0 (or undefined)', () => {
    expect(filePathToSrcVersioned('/tmp/x.html')).toBe('local-file:///tmp/x.html')
    expect(filePathToSrcVersioned('/tmp/x.html', 0)).toBe('local-file:///tmp/x.html')
  })

  it('appends ?v= for a positive version', () => {
    expect(filePathToSrcVersioned('/tmp/x.html', 3)).toBe('local-file:///tmp/x.html?v=3')
  })

  it('uses & when the url already has a query', () => {
    expect(filePathToSrcVersioned('https://x.com/a?q=1', 2)).toBe('https://x.com/a?q=1&v=2')
  })
})
