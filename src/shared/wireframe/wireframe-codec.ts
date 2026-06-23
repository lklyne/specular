// Codec between `.wireframe.json` file content and the value stored in the
// Y.Doc `wireframes` map.
//
// Granularity A1 (per plan 2.1): the Y value is the *canonical JSON string* of
// the wireframe — one string per file-entity, one transaction per op. `seed`
// canonicalizes a file's on-disk text for storage; `extract` reads it back out
// to write to disk. For valid trees the pair round-trips exactly:
//
//   extract(seed(text)) === serializeWireframeFile(parse(text))
//
// so a file already written in canonical form satisfies `extract(seed(t)) === t`.
//
// Pure and side-effect-free (src/shared rules): no fs, no Electron. The main
// process owns the disk I/O; this module only shapes the strings.

import { validateWireframe } from './wireframe-ops'
import type { WireframeFile } from './wireframe-types'

/** Canonical on-disk / in-doc serialization: 2-space JSON, matching the rest of the app. */
export function serializeWireframeFile(file: WireframeFile): string {
  return JSON.stringify(file, null, 2)
}

/**
 * Parse and structurally validate wireframe file text. Throws on malformed JSON
 * or an invalid tree — the validating boundary the op/apply path relies on.
 */
export function parseWireframeFile(text: string): WireframeFile {
  let parsed: unknown
  try {
    parsed = JSON.parse(text)
  } catch (err) {
    throw new Error(`Wireframe is not valid JSON: ${(err as Error).message}`)
  }
  const result = validateWireframe(parsed)
  if (!result.ok) {
    throw new Error(`Invalid wireframe: ${result.errors.join('; ')}`)
  }
  return parsed as WireframeFile
}

/** File text → canonical Y value. Validates; throws for invalid trees. */
export function seedWireframeContent(fileText: string): string {
  return serializeWireframeFile(parseWireframeFile(fileText))
}

/** Y value → file text. The stored value is already canonical JSON. */
export function extractWireframeContent(stored: string): string {
  return stored
}
