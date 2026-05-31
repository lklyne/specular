import { describe, expect, it } from 'vitest'
import { hitTest, type HitInputs } from '../../src/shared/hit-test'
import type {
  CanvasSceneDrawingEntity,
  CanvasSceneEntity,
  CanvasScenePageEntity,
  CanvasSceneTextEntity,
  CanvasSceneGroupEntity,
} from '../../src/shared/types'

// --- Fixtures ---

function page(overrides: Partial<CanvasScenePageEntity> & { id: string }): CanvasScenePageEntity {
  const screenX = overrides.screenX ?? 200
  const screenY = overrides.screenY ?? 200
  const screenWidth = overrides.screenWidth ?? 400
  const screenHeight = overrides.screenHeight ?? 300
  return {
    kind: 'page',
    id: overrides.id,
    label: overrides.label ?? 'page',
    url: overrides.url ?? 'https://example.com',
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    isCustomSize: false,
    browserSizeMode: 'fill',
    canvasX: 0,
    canvasY: 0,
    width: screenWidth,
    height: screenHeight,
    presetIndex: 0,
    linked: false,
    screenX,
    screenY,
    screenWidth,
    screenHeight,
    ...overrides,
  }
}

function text(id: string, screenX: number, screenY: number, w = 100, h = 40): CanvasSceneTextEntity {
  return {
    kind: 'text',
    id,
    text: 'hello',
    color: '#000',
    canvasX: 0,
    canvasY: 0,
    width: w,
    height: h,
    screenX,
    screenY,
    screenWidth: w,
    screenHeight: h,
  }
}

function group(id: string, screenX: number, screenY: number, w = 600, h = 500): CanvasSceneGroupEntity {
  return {
    kind: 'group',
    id,
    label: 'g',
    canvasX: 0,
    canvasY: 0,
    width: w,
    height: h,
    screenX,
    screenY,
    screenWidth: w,
    screenHeight: h,
    layoutMode: 'freeform',
    managedLayout: false,
    entityIds: [],
  }
}

function drawing(id: string, screenX: number, screenY: number, w = 100, h = 60): CanvasSceneDrawingEntity {
  return {
    kind: 'drawing',
    id,
    canvasX: 0,
    canvasY: 0,
    width: w,
    height: h,
    screenX,
    screenY,
    screenWidth: w,
    screenHeight: h,
    strokes: [],
  }
}

function inputs(entities: CanvasSceneEntity[], selectedEntityIds: string[] = []): HitInputs {
  return { entities, edges: [], selectedEntityIds, zoom: 1 }
}

// --- Collision class tests ---

describe('hit-test — issue #41 anchor near chrome', () => {
  // Page at screen (200,200), 400×300. Chrome strip is 36px above the page
  // body, occupying y ∈ [164, 200). The top anchor's hit rect sits 4px
  // above the page top, height 24px → y ∈ [172, 196). Heavy overlap with
  // chrome y ∈ [164, 200). The bug: anchor wins. The fix: chrome wins.
  const f = page({ id: 'f1', screenX: 200, screenY: 200 })

  it('chrome wins over top anchor when page is selected', () => {
    const result = hitTest(inputs([f], ['f1']), { x: 400, y: 185 })
    expect(result.layer).toBe('chrome')
    expect(result.payload).toMatchObject({ kind: 'chrome', entityId: 'f1' })
  })

  it('an anchor click clear of the chrome still hits the anchor', () => {
    // Right-side anchor: x ∈ [604, 628], y ∈ [338, 362]. Well clear of chrome.
    const result = hitTest(inputs([f], ['f1']), { x: 615, y: 350 })
    expect(result.layer).toBe('anchors')
    expect(result.payload).toMatchObject({ kind: 'anchor', side: 'right' })
  })
})

describe('hit-test — resize handles vs body', () => {
  // Page selected, resize handle at NE corner = (600, 200). Click on the
  // corner should resize, not enter focus.
  const f = page({ id: 'f1', screenX: 200, screenY: 200 })

  it('resize handle wins over page body at the corner', () => {
    const result = hitTest(inputs([f], ['f1']), { x: 600, y: 200 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', handle: 'ne' })
  })

  it('clicking on the visible NE handle (offset by page outline padding) resizes', () => {
    // Page outline pads the NE corner outward by 2px; the visible handle
    // is centered there, so a real user click typically lands a few pixels
    // past the entity edge. Pre-fix this fell through to background → marquee.
    // NE handle rect: x ∈ [596, 608], y ∈ [192, 204]. Click (606,194) is inside.
    const result = hitTest(inputs([f], ['f1']), { x: 606, y: 194 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', handle: 'ne' })
  })

  it('clicking on the top edge handle far from the midpoint still resizes', () => {
    // The visible top-edge resize handle spans the full edge corner-to-corner.
    // Pre-fix the hit-test was a 12×12 patch at the midpoint only, so a click
    // 30px from a corner fell through to background.
    const result = hitTest(inputs([f], ['f1']), { x: 230, y: 195 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', handle: 'n' })
  })

  it('clicking deep in the body without selection enters focus', () => {
    const result = hitTest(inputs([f], []), { x: 400, y: 350 })
    expect(result.layer).toBe('body')
    expect(result.payload).toMatchObject({ kind: 'page-body', entityId: 'f1' })
  })
})

describe('hit-test — body kind dispatches', () => {
  it('page body returns page-body intent (focus)', () => {
    const f = page({ id: 'f1' })
    const result = hitTest(inputs([f]), { x: 400, y: 350 })
    expect(result.payload).toEqual({ kind: 'page-body', entityId: 'f1' })
  })

  it('text body returns entity-body intent (select)', () => {
    const t = text('t1', 200, 200)
    const result = hitTest(inputs([t]), { x: 250, y: 220 })
    expect(result.payload).toEqual({ kind: 'entity-body', entityId: 't1', entityKind: 'text' })
  })
})

describe('hit-test — group containment', () => {
  // Group spans (100,100) → (700,600). Text inside at (300,300), 100×40.
  // Click inside the text → text body. Click in the group but not in the
  // text → group body.
  const g = group('g1', 100, 100, 600, 500)
  const t = text('t1', 300, 300)

  it('clicking on a member entity selects the member, not the group', () => {
    const result = hitTest(inputs([g, t]), { x: 350, y: 320 })
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 't1' })
  })

  it('clicking inside the group but outside its members selects the group', () => {
    const result = hitTest(inputs([g, t]), { x: 600, y: 550 })
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 'g1', entityKind: 'group' })
  })
})

describe('hit-test — background fallback', () => {
  it('returns background when no entity is hit', () => {
    const result = hitTest(inputs([page({ id: 'f1' })]), { x: 10, y: 10 })
    expect(result.payload).toEqual({ kind: 'background' })
    expect(result.layer).toBe('background')
  })
})

describe('hit-test — chrome only on chrome-bearing entities', () => {
  it('text entities have no chrome strip', () => {
    const t = text('t1', 200, 200, 100, 40)
    // 36px above text top: y ∈ [164, 200). No chrome should be present.
    const result = hitTest(inputs([t]), { x: 250, y: 180 })
    expect(result.payload).toEqual({ kind: 'background' })
  })
})

describe('hit-test — body z-order (front-to-back)', () => {
  // entityOrder semantics: array order is back-to-front (paint order — the
  // last item paints on top, matching JSON Canvas v1.0). The hit-test must
  // walk the body layer front-to-back so a sticky painted over a page
  // resolves to the sticky, not the page body underneath.
  //
  // Page body at (200,200), 400×300. Sticky at (300,300), 100×40.
  // The sticky is fully inside the page.

  it('sticky declared front (after page in entities) wins over page body', () => {
    const f = page({ id: 'f1', screenX: 200, screenY: 200 })
    const t = text('t1', 300, 300)
    // Page first (back), text after (front).
    const result = hitTest(inputs([f, t]), { x: 320, y: 320 })
    expect(result.layer).toBe('body')
    expect(result.payload).toEqual({
      kind: 'entity-body',
      entityId: 't1',
      entityKind: 'text',
    })
  })

  it('reverse z-order (sticky behind page) returns page-body', () => {
    const f = page({ id: 'f1', screenX: 200, screenY: 200 })
    const t = text('t1', 300, 300)
    // Text first (back), page after (front).
    const result = hitTest(inputs([t, f]), { x: 320, y: 320 })
    expect(result.layer).toBe('body')
    expect(result.payload).toEqual({ kind: 'page-body', entityId: 'f1' })
  })

  it('two non-group entities — last in entities wins (front)', () => {
    // Two stacked text entities both covering the click point.
    const t1 = text('t1', 200, 200, 200, 200)
    const t2 = text('t2', 200, 200, 200, 200)
    // t2 declared after t1 → front.
    const result = hitTest(inputs([t1, t2]), { x: 300, y: 300 })
    expect(result.payload).toEqual({
      kind: 'entity-body',
      entityId: 't2',
      entityKind: 'text',
    })
    // Reversed declaration order swaps which wins.
    const reversed = hitTest(inputs([t2, t1]), { x: 300, y: 300 })
    expect(reversed.payload).toEqual({
      kind: 'entity-body',
      entityId: 't1',
      entityKind: 'text',
    })
  })

  it('group containment still wins regardless of declared order', () => {
    // Even if the group is declared LAST (front-most by paint order), a
    // member entity inside the group must hit first — groups are containers,
    // their hit-test runs after non-group bodies.
    const g = group('g1', 100, 100, 600, 500)
    const t = text('t1', 300, 300)
    // Group declared after text → would be "front" by paint order, but
    // groups sit at the bottom of the hit-test priority within the body
    // layer.
    const result = hitTest(inputs([t, g]), { x: 350, y: 320 })
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 't1' })
  })
})

describe('hit-test — group resize handles', () => {
  // Group at (100,100), 600×500. SE corner at (700, 600).
  // With 2px outline padding, SE handle rect: x ∈ [696, 708], y ∈ [596, 608].
  const g = group('g1', 100, 100, 600, 500)

  it('resize handle wins at the SE corner when group is selected', () => {
    const result = hitTest(inputs([g], ['g1']), { x: 700, y: 600 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', handle: 'se', entityId: 'g1' })
  })

  it('clicking 2px past the SE corner hits the handle (2px outline padding)', () => {
    // Handle rect SE: x ∈ [696, 708], y ∈ [596, 608]. Click (702, 602) is inside.
    const result = hitTest(inputs([g], ['g1']), { x: 702, y: 602 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', handle: 'se' })
  })
})

describe('hit-test — multi-selection resize handles', () => {
  // Two text entities at (100,100,50,50) and (200,200,80,40) → bbox spans
  // (100,100) to (280,240). Multi-bbox padding is 8 → outer corners at
  // (92,92) and (288,248).
  const t1 = text('t1', 100, 100, 50, 50)
  const t2 = text('t2', 200, 200, 80, 40)

  it('emits a multi-resize handle on the bbox SE corner when 2+ entities are selected', () => {
    const result = hitTest(inputs([t1, t2], ['t1', 't2']), { x: 288, y: 248 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toEqual({ kind: 'multi-resize-handle', handle: 'se' })
  })

  it('emits a multi-resize handle on the bbox NW corner', () => {
    const result = hitTest(inputs([t1, t2], ['t1', 't2']), { x: 92, y: 92 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toEqual({ kind: 'multi-resize-handle', handle: 'nw' })
  })

  it('per-entity handles are suppressed in multi-select (no entityId on the payload)', () => {
    // Click directly on t1's NW corner — without multi-select this is a
    // per-entity resize handle. With multi-select it must miss (the
    // multi-bbox NW corner is at 92,92, well off from (100,100)) and fall
    // through to the body layer.
    const result = hitTest(inputs([t1, t2], ['t1', 't2']), { x: 100, y: 100 })
    expect(result.layer).not.toBe('resize-handles')
  })

  it('falls through to per-entity handles when only one entity is selected', () => {
    const result = hitTest(inputs([t1, t2], ['t1']), { x: 100, y: 100 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', entityId: 't1' })
  })

  it('skips the multi-bbox when fewer than two non-group entities are selected', () => {
    // Selection includes a group + one entity — multi-bbox needs 2+
    // non-group entities, so this should fall to per-entity handles only.
    const g = group('g1', 0, 0, 50, 50)
    const result = hitTest(inputs([t1, g], ['t1', 'g1']), { x: 100, y: 100 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', entityId: 't1' })
  })
})

describe('hit-test — selected drawing beats page chrome and page body (issue #123)', () => {
  // Page at (200, 200), 400×300. Chrome strip occupies y ∈ [164, 200).
  // Drawing at (220, 170), 100×80. Drawing body: x ∈ [220,320], y ∈ [170,250].
  // This overlaps the page chrome at y ∈ [170, 200) and the page body at
  // y ∈ [200, 250]. Drawings render in aboveView above all page elements.
  const p = page({ id: 'p1', screenX: 200, screenY: 200 })
  const d = drawing('d1', 220, 170, 100, 80)

  it('selected drawing beats page chrome when overlapping (drag-from-chrome-area)', () => {
    // y=185 is inside the page chrome strip and inside the drawing body.
    const result = hitTest(inputs([p, d], ['d1']), { x: 260, y: 185 })
    expect(result.layer).toBe('body')
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 'd1', entityKind: 'drawing' })
  })

  it('selected drawing beats page body when overlapping (drag-from-body-area)', () => {
    // y=215 is inside the page body and inside the drawing body.
    const result = hitTest(inputs([p, d], ['d1']), { x: 260, y: 215 })
    expect(result.layer).toBe('body')
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 'd1', entityKind: 'drawing' })
  })

  it('page chrome still wins where no selected drawing overlaps', () => {
    // x=500 is inside the page chrome but outside the drawing (x ∈ [220,320]).
    const result = hitTest(inputs([p, d], ['d1']), { x: 500, y: 185 })
    expect(result.layer).toBe('chrome')
    expect(result.payload).toMatchObject({ kind: 'chrome', entityId: 'p1' })
  })

  it('unselected drawing does not get the priority boost — page chrome wins', () => {
    // Same geometry, but drawing is NOT selected.
    const result = hitTest(inputs([p, d], []), { x: 260, y: 185 })
    expect(result.layer).toBe('chrome')
    expect(result.payload).toMatchObject({ kind: 'chrome', entityId: 'p1' })
  })

  it('resize handles of the selected drawing still win over drawing body', () => {
    // SE resize handle of drawing: x ≈ 320+2+6, y ≈ 250+2+6 (2px outline pad for drawing).
    // Handle half = RESIZE_HANDLE_HIT_PX/2 ≈ 6. Corner at (320+2, 250+2) = (322, 252).
    // Click at (322, 252) — the handle should win, not the drawing body.
    const result = hitTest(inputs([p, d], ['d1']), { x: 322, y: 252 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', entityId: 'd1' })
  })

  it('selected drawing body does not leak into areas it does not cover', () => {
    // y=350 is inside the page body only, not the drawing. Should return page-body.
    const result = hitTest(inputs([p, d], ['d1']), { x: 260, y: 350 })
    expect(result.layer).toBe('body')
    expect(result.payload).toMatchObject({ kind: 'page-body', entityId: 'p1' })
  })
})

describe('hit-test — auto-layout reorder dots (ADR 0015)', () => {
  // Managed-row group g1 with two text children c1, c2. Child c1 at screen
  // (220,220) 100×40 → center (270,240); reorder hit square is 28px centered →
  // x ∈ [256,284], y ∈ [226,254].
  const managedGroup = (entityIds: string[]): CanvasSceneGroupEntity => ({
    ...group('g1', 200, 200, 600, 200),
    layoutMode: 'row',
    managedLayout: true,
    entityIds,
  })
  const c1 = text('c1', 220, 220, 100, 40)
  const c2 = text('c2', 400, 220, 100, 40)
  const center = { x: 270, y: 240 }

  it('emits a reorder handle at the child center when the group is selected', () => {
    const result = hitTest(
      { entities: [managedGroup(['c1', 'c2']), c1, c2], edges: [], selectedEntityIds: [], selectedGroupId: 'g1', zoom: 1 },
      center,
    )
    expect(result.layer).toBe('reorder-handle')
    expect(result.payload).toMatchObject({ kind: 'reorder-handle', entityId: 'c1' })
  })

  it('emits a reorder handle when the child itself is selected', () => {
    const result = hitTest(inputs([managedGroup(['c1', 'c2']), c1, c2], ['c1']), center)
    expect(result.payload).toMatchObject({ kind: 'reorder-handle', entityId: 'c1' })
  })

  it('does not emit a reorder handle when neither group nor child is selected', () => {
    const result = hitTest(inputs([managedGroup(['c1', 'c2']), c1, c2], []), center)
    expect(result.payload.kind).not.toBe('reorder-handle')
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 'c1' })
  })

  it('does not emit a reorder handle for an unmanaged group', () => {
    const freeform: CanvasSceneGroupEntity = { ...group('g1', 200, 200, 600, 200), entityIds: ['c1', 'c2'] }
    const result = hitTest(
      { entities: [freeform, c1, c2], edges: [], selectedEntityIds: [], selectedGroupId: 'g1', zoom: 1 },
      center,
    )
    expect(result.payload.kind).not.toBe('reorder-handle')
  })

  it('child resize handle still wins over the reorder dot at the corner', () => {
    // c1 NE corner ≈ (320, 220) (2px outline pad). The reorder dot is below
    // resize handles in priority, so a corner click resizes.
    const result = hitTest(inputs([managedGroup(['c1', 'c2']), c1, c2], ['c1']), { x: 320, y: 220 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', entityId: 'c1' })
  })
})

describe('hit-test — selection reorder door (ADR 0015 D7)', () => {
  // Loose (ungrouped) equal-gap row: three 100×40 texts, gap 50 along x. Canvas
  // and screen coords coincide (zoom 1) so the detector and the dot center agree.
  // Centers at screen x = 250, 400, 550; y = 220.
  function rowText(id: string, x: number): CanvasSceneTextEntity {
    return { ...text(id, x, 200, 100, 40), canvasX: x, canvasY: 200 }
  }
  const e1 = rowText('e1', 200)
  const e2 = rowText('e2', 350) // gap 50 → equal
  const center1 = { x: 250, y: 220 }

  it('emits a reorder handle on an equal-gap multi-selection (no group)', () => {
    const result = hitTest(inputs([e1, e2, rowText('e3', 500)], ['e1', 'e2', 'e3']), center1)
    expect(result.layer).toBe('reorder-handle')
    expect(result.payload).toMatchObject({ kind: 'reorder-handle', entityId: 'e1' })
  })

  it('exposes no reorder hit target on an unequal-gap selection', () => {
    // e3 sits far past e2 — the second gap dwarfs the first, so it is not a row.
    const result = hitTest(inputs([e1, e2, rowText('e3', 900)], ['e1', 'e2', 'e3']), center1)
    expect(result.payload.kind).not.toBe('reorder-handle')
  })

  it('exposes no reorder hit target on a single selection', () => {
    const result = hitTest(inputs([e1, e2], ['e1']), center1)
    expect(result.payload.kind).not.toBe('reorder-handle')
  })
})
