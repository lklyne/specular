import { describe, it, expect } from 'vitest'
import { detectSmartPaste } from '../../src/renderer/canvas-bg/entity-renderers/markdown-smart-paste'

function mockPasteData(types: string[], data: Record<string, string>) {
  return {
    types,
    getData: (type: string) => data[type] ?? '',
  }
}

const MINIMAL_SVG = '<svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10"/></svg>'

describe('detectSmartPaste', () => {
  describe('SVG detection', () => {
    it('detects SVG via image/svg+xml MIME type', () => {
      const result = detectSmartPaste(
        mockPasteData(['image/svg+xml', 'text/plain'], {
          'image/svg+xml': MINIMAL_SVG,
          'text/plain': MINIMAL_SVG,
        }),
      )
      expect(result).toEqual({ lang: 'svg', text: MINIMAL_SVG })
    })

    it('falls back to text/plain when image/svg+xml slot is empty', () => {
      const result = detectSmartPaste(
        mockPasteData(['image/svg+xml', 'text/plain'], {
          'image/svg+xml': '',
          'text/plain': MINIMAL_SVG,
        }),
      )
      expect(result).toEqual({ lang: 'svg', text: MINIMAL_SVG })
    })

    it('detects SVG from plain text structural sniff', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': MINIMAL_SVG }),
      )
      expect(result).toEqual({ lang: 'svg', text: MINIMAL_SVG })
    })

    it('does not detect partial SVG missing closing tag', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], {
          'text/plain': '<svg xmlns="http://www.w3.org/2000/svg"><rect/>',
        }),
      )
      expect(result).toBeNull()
    })

    it('does not detect SVG-like prose (no opening <svg tag)', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': '<div>hello</div>' }),
      )
      expect(result).toBeNull()
    })
  })

  describe('JSON detection', () => {
    it('detects JSON object and pretty-prints it', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': '{"foo":"bar","n":1}' }),
      )
      expect(result).toEqual({
        lang: 'json',
        text: JSON.stringify({ foo: 'bar', n: 1 }, null, 2),
      })
    })

    it('detects JSON array', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': '[1,2,3]' }),
      )
      expect(result).toEqual({ lang: 'json', text: JSON.stringify([1, 2, 3], null, 2) })
    })

    it('detects JSON via application/json MIME type', () => {
      const result = detectSmartPaste(
        mockPasteData(['application/json'], {
          'application/json': '{"key":"value"}',
        }),
      )
      expect(result).toEqual({
        lang: 'json',
        text: JSON.stringify({ key: 'value' }, null, 2),
      })
    })

    it('does not detect invalid JSON even when braces match', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': '{not: valid json}' }),
      )
      expect(result).toBeNull()
    })

    it('does not detect bare JSON primitive (not object or array)', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': '"just a string"' }),
      )
      expect(result).toBeNull()
    })
  })

  describe('HTML detection', () => {
    it('detects full HTML document starting with DOCTYPE', () => {
      const html = '<!DOCTYPE html><html><head></head><body>hello</body></html>'
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': html }),
      )
      expect(result).toEqual({ lang: 'html', text: html })
    })

    it('detects full HTML document starting with <html', () => {
      const html = '<html><body>hello</body></html>'
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': html }),
      )
      expect(result).toEqual({ lang: 'html', text: html })
    })

    it('does not detect HTML snippet (generic element, not full document)', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], {
          'text/plain': '<div class="foo"><p>paragraph</p></div>',
        }),
      )
      expect(result).toBeNull()
    })
  })

  describe('prose and no-match cases', () => {
    it('returns null for plain prose', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': 'This is just some plain text.' }),
      )
      expect(result).toBeNull()
    })

    it('returns null for empty clipboard', () => {
      const result = detectSmartPaste(
        mockPasteData(['text/plain'], { 'text/plain': '' }),
      )
      expect(result).toBeNull()
    })

    it('returns null when no text/plain is available', () => {
      const result = detectSmartPaste(mockPasteData([], {}))
      expect(result).toBeNull()
    })
  })
})
