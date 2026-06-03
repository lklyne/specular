import { describe, expect, it } from 'vitest'
import type { WireframeFile } from '../../src/shared/wireframe/wireframe-types'
import {
  extractWireframeContent,
  parseWireframeFile,
  seedWireframeContent,
  serializeWireframeFile,
} from '../../src/shared/wireframe/wireframe-codec'

// The file<->Y codec for the A1 JSON-string projection (plan 3.0b). The Y value
// is the canonical JSON text; seed canonicalizes a file's content for storage,
// extract reads it back. For valid trees the pair round-trips exactly.

function sampleFile(): WireframeFile {
  return {
    version: '1.0',
    theme: 'light',
    root: {
      id: 'root',
      type: 'frame',
      direction: 'vertical',
      gap: 8,
      children: [
        { id: 'title', type: 'text', text: 'Hello', level: 'h1' },
        {
          id: 'row',
          type: 'frame',
          direction: 'horizontal',
          children: [
            { id: 'btn-a', type: 'button', text: 'A', variant: 'primary' },
            { id: 'agree', type: 'checkbox', label: 'Agree', checked: false },
          ],
        },
      ],
    },
  }
}

describe('wireframe-codec', () => {
  it('round-trips a valid tree: extract(seed(json)) === json for canonical input', () => {
    const json = serializeWireframeFile(sampleFile())
    expect(extractWireframeContent(seedWireframeContent(json))).toBe(json)
  })

  it('seed canonicalizes loosely-formatted but valid input', () => {
    const canonical = serializeWireframeFile(sampleFile())
    const messy = JSON.stringify(sampleFile()) // no indentation
    expect(seedWireframeContent(messy)).toBe(canonical)
    // And canonicalizing is idempotent.
    expect(extractWireframeContent(seedWireframeContent(messy))).toBe(canonical)
  })

  it('parse rejects malformed JSON', () => {
    expect(() => parseWireframeFile('{not json')).toThrow(/not valid JSON/)
  })

  it('parse rejects a structurally invalid tree', () => {
    const badRoot = JSON.stringify({ version: '1.0', root: { id: 'r', type: 'text', text: 'x' } })
    expect(() => parseWireframeFile(badRoot)).toThrow(/Invalid wireframe/)
  })
})
