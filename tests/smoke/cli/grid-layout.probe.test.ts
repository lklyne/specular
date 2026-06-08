import { describe, it, expect } from 'vitest'
import { runCli } from './cli-probe-utils'

// Grid layout correctness (charter "Prescribed canvas workflows"): an agent lays
// out many items as a grid via the CLI and expects them not to overlap. This is
// the failure mode that only shows up at scale — a handful of items rarely
// collide, a dozen do. The probe drives the real layout engine through
// `specular upsert --json` (grid directive), reads geometry back from
// `specular workspace`, and asserts the geometric property directly.
//
// Entities are short-text notes with explicit width/height: deterministic
// footprints, no webview or renderer measurement needed, and the workspace graph
// reports canvasX/canvasY/width/height for them.

interface Rect {
  id: string
  x: number
  y: number
  w: number
  h: number
}

interface WsEntity {
  id: string
  kind: string
  canvasX?: number
  canvasY?: number
  width?: number
  height?: number
}

/** Strict AABB overlap with a small epsilon so edge-to-edge touching (gap 0) is allowed. */
function rectsOverlap(a: Rect, b: Rect, eps = 0.5): boolean {
  return a.x + a.w - eps > b.x && b.x + b.w - eps > a.x && a.y + a.h - eps > b.y && b.y + b.h - eps > a.y
}

function firstOverlap(rects: Rect[]): [Rect, Rect] | null {
  for (let i = 0; i < rects.length; i++) {
    for (let j = i + 1; j < rects.length; j++) {
      if (rectsOverlap(rects[i], rects[j])) return [rects[i], rects[j]]
    }
  }
  return null
}

/** Lay items out with a grid directive in one call; return the created ids. */
function gridUpsert(items: Array<Record<string, unknown>>, layout: Record<string, unknown>): Set<string> {
  const r = runCli(['upsert', '--json'], { input: JSON.stringify({ layout, items }) })
  expect(r.code, `upsert failed: ${r.stderr}`).toBe(0)
  const created = (r.json as { created?: string[] } | undefined)?.created ?? []
  expect(created.length, `expected ${items.length} created, got ${JSON.stringify(r.json)}`).toBe(items.length)
  return new Set(created)
}

/** Read geometry back for exactly the entities we created (scoped by id). */
function rectsForIds(ids: Set<string>): Rect[] {
  const ws = runCli(['workspace'])
  expect(ws.code, ws.stderr).toBe(0)
  const entities = (ws.json as { entities?: WsEntity[] } | undefined)?.entities ?? []
  return entities
    .filter((e) => ids.has(e.id))
    .map((e) => ({ id: e.id, x: e.canvasX ?? NaN, y: e.canvasY ?? NaN, w: e.width ?? NaN, h: e.height ?? NaN }))
}

function distinct(values: number[]): number[] {
  return [...new Set(values.map((v) => Math.round(v)))]
}

describe('cli probe: grid layout does not overlap at scale', () => {
  it('a 12-item uniform grid has no overlaps and a real grid structure', () => {
    const run = `grid-a-${Date.now()}`
    const items = Array.from({ length: 12 }, (_, i) => ({
      kind: 'text',
      text: `${run}-${i}`,
      width: 200,
      height: 150,
    }))
    const ids = gridUpsert(items, { kind: 'grid', cols: 4, gap: 24, originX: 0, originY: 0 })
    const rects = rectsForIds(ids)

    // Every created item is present with real geometry.
    expect(rects.length).toBe(12)
    for (const r of rects) {
      expect(Number.isFinite(r.x) && Number.isFinite(r.y), `entity ${r.id} missing position`).toBe(true)
      expect(r.w, `entity ${r.id} has no width`).toBeGreaterThan(0)
      expect(r.h, `entity ${r.id} has no height`).toBeGreaterThan(0)
    }

    // The core property: no two items overlap.
    const clash = firstOverlap(rects)
    expect(clash, clash ? `overlap between ${clash[0].id} and ${clash[1].id}` : '').toBeNull()

    // It is actually a grid, not a collapsed pile or a single column:
    // 12 items in 4 columns → 4 distinct x tracks and 3 distinct y rows.
    expect(distinct(rects.map((r) => r.x)).length).toBe(4)
    expect(distinct(rects.map((r) => r.y)).length).toBe(3)
  })

  it('a grid of many mixed-size items still leaves no overlaps', () => {
    const run = `grid-b-${Date.now()}`
    // Alternating footprints — the case where naive packing collides.
    const items = Array.from({ length: 9 }, (_, i) => ({
      kind: 'text',
      text: `${run}-${i}`,
      width: i % 2 === 0 ? 200 : 320,
      height: i % 3 === 0 ? 150 : 240,
    }))
    const ids = gridUpsert(items, { kind: 'grid', cols: 3, gap: 24, originX: 0, originY: 0 })
    const rects = rectsForIds(ids)

    expect(rects.length).toBe(9)
    const clash = firstOverlap(rects)
    expect(clash, clash ? `overlap between ${clash[0].id} and ${clash[1].id}` : '').toBeNull()
  })
})
