import { describe, expect, it } from 'vitest'
import { hitTest, type HitInputs } from '../../src/shared/hit-test'
import type {
  CanvasSceneDrawingEntity,
  CanvasSceneEntity,
  CanvasScenePageEntity,
  CanvasSceneShapeEntity,
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
    canvasX: 0,
    canvasY: 0,
    width: screenWidth,
    height: screenHeight,
    presetIndex: 0,
    synced: false,
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

function shape(id: string, screenX: number, screenY: number, w = 100, h = 80): CanvasSceneShapeEntity {
  return {
    kind: 'shape',
    id,
    shapeKind: 'rectangle',
    text: '',
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

function inputs(
  entities: CanvasSceneEntity[],
  selectedEntityIds: string[] = [],
  overrides: Partial<HitInputs> = {},
): HitInputs {
  return { entities, edges: [], selectedEntityIds, zoom: 1, ...overrides }
}

// --- Collision class tests ---

describe('hit-test — top-edge anchors (no chrome band)', () => {
  // Page at screen (200,200), 400×300. The top anchor's hit rect sits 4px
  // above the page top, height 24px → y ∈ [172, 196). With the chrome header
  // retired, nothing shadows the anchor: it wins directly on select/hover.
  const f = page({ id: 'f1', screenX: 200, screenY: 200 })

  it('top anchor wins when the page is hovered but not selected', () => {
    const result = hitTest(inputs([f], [], { hoveredEntityId: 'f1' }), { x: 400, y: 185 })
    expect(result.layer).toBe('anchors')
    expect(result.payload).toMatchObject({ kind: 'anchor', entityId: 'f1', side: 'top' })
  })

  it('top anchor wins when the page is selected', () => {
    const result = hitTest(inputs([f], ['f1']), { x: 400, y: 185 })
    expect(result.layer).toBe('anchors')
    expect(result.payload).toMatchObject({ kind: 'anchor', side: 'top' })
  })

  it('a point above the body with no eligible anchor falls through to background', () => {
    // Page neither selected nor hovered → no anchors → nothing above the body.
    const result = hitTest(inputs([f], []), { x: 400, y: 185 })
    expect(result.layer).toBe('background')
  })

  it('a right-side anchor still hits the anchor', () => {
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

  it('clicking a few px past the NE corner still resizes (12px hit rect)', () => {
    // The handle centers on the 1px-padded outline corner, so a real user
    // click can land a few pixels past the entity edge. Pre-fix this fell
    // through to background → marquee.
    // NE handle rect: x ∈ [595, 607], y ∈ [193, 205]. Click (606,194) is inside.
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

describe('hit-test — item overlapping a page top edge wins (issue #312)', () => {
  // The retired chrome header used to occupy an invisible 36px band above the
  // page body and outrank overlapping items, stealing their drags. With chrome
  // gone, a shape straddling the page's top edge hits as a normal front body.
  // Page at (200,200), 400×300 → body y ∈ [200, 500].
  // Shape at (240,170), 100×80 → body y ∈ [170, 250], declared after the page.
  const p = page({ id: 'p1', screenX: 200, screenY: 200 })
  const s = shape('s1', 240, 170, 100, 80)

  it('shape over the page top edge wins where the old chrome band used to steal', () => {
    // y=185 sits in the old chrome band and inside the shape body.
    const result = hitTest(inputs([p, s]), { x: 280, y: 185 })
    expect(result.layer).toBe('body')
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 's1', entityKind: 'shape' })
  })

  it('shape wins over the page body where they overlap below the edge', () => {
    // y=230 is inside both the shape and the page body.
    const result = hitTest(inputs([p, s]), { x: 280, y: 230 })
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 's1' })
  })
})

describe('hit-test — body z-order (front-to-back)', () => {
  // entityOrder semantics within a physical plane: array order is
  // back-to-front (paint order — the last item paints on top, matching JSON
  // Canvas v1.0). Across planes, Notes always paint in aboveView over page
  // WCVs, so a visible sticky must beat a page regardless of their flat
  // persisted order.
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

  it('sticky still wins when flat entityOrder ranks the page in front', () => {
    const f = page({ id: 'f1', screenX: 200, screenY: 200 })
    const t = text('t1', 300, 300)
    // Text first, page after in the flat order. They occupy different physical
    // planes, so this cannot make the page visually cover the sticky.
    const result = hitTest(inputs([t, f]), { x: 320, y: 320 })
    expect(result.layer).toBe('body')
    expect(result.payload).toEqual({
      kind: 'entity-body',
      entityId: 't1',
      entityKind: 'text',
    })
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
  // 1px outline padding: SE handle rect is x ∈ [695, 707], y ∈ [595, 607].
  const g = group('g1', 100, 100, 600, 500)

  it('resize handle wins at the SE corner when group is selected', () => {
    const result = hitTest(inputs([g], ['g1']), { x: 700, y: 600 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', handle: 'se', entityId: 'g1' })
  })

  it('clicking 2px past the SE corner still hits the handle (12px hit rect)', () => {
    // Handle rect SE: x ∈ [695, 707], y ∈ [595, 607]. Click (702, 602) is inside.
    const result = hitTest(inputs([g], ['g1']), { x: 702, y: 602 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', handle: 'se' })
  })

  it('does not expose invisible per-entity handles when a group is batch-selected', () => {
    const sibling = text('t1', 800, 100, 100, 100)
    const result = hitTest(inputs([g, sibling], ['g1', 't1']), { x: 700, y: 600 })
    expect(result.payload.kind).not.toBe('resize-handle')
  })

  it('emits multi-resize handles over the operand bbox when a group is batch-selected', () => {
    // Group rect (100,100,600,500) + sibling text at (800,100,100,100) →
    // operand bbox spans (100,100)–(900,600), the group's own rect included.
    // With 1px padding the SE corner handle centers at (901, 601); the 12px
    // hit rect covers (900, 600).
    const child = text('c1', 150, 150, 100, 100)
    const sibling = text('t1', 800, 100, 100, 100)
    const result = hitTest(
      inputs([g, child, sibling], ['g1', 't1'], {
        selectionOperandIds: ['g1', 'c1', 't1'],
      }),
      { x: 900, y: 600 },
    )
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toEqual({ kind: 'multi-resize-handle', handle: 'se' })
  })
})

describe('hit-test — multi-selection resize handles', () => {
  // Two text entities at (100,100,50,50) and (200,200,80,40) → bbox spans
  // (100,100) to (280,240). The box sits 1px outside — handles center on
  // the padded corners, and the 12px hit rects cover the corners themselves.
  const t1 = text('t1', 100, 100, 50, 50)
  const t2 = text('t2', 200, 200, 80, 40)

  it('emits a multi-resize handle on the bbox SE corner when 2+ entities are selected', () => {
    const result = hitTest(inputs([t1, t2], ['t1', 't2']), { x: 280, y: 240 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toEqual({ kind: 'multi-resize-handle', handle: 'se' })
  })

  it('emits a multi-resize handle on the bbox NW corner', () => {
    const result = hitTest(inputs([t1, t2], ['t1', 't2']), { x: 100, y: 100 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toEqual({ kind: 'multi-resize-handle', handle: 'nw' })
  })

  it('per-entity handles are suppressed in multi-select (no entityId on the payload)', () => {
    // Click directly on t1's SE corner (150,150) — without multi-select this
    // is a per-entity resize handle. With multi-select it must miss (the
    // point is interior to the bbox, away from every multi handle) and fall
    // through to the body layer.
    const result = hitTest(inputs([t1, t2], ['t1', 't2']), { x: 150, y: 150 })
    expect(result.layer).not.toBe('resize-handles')
  })

  it('falls through to per-entity handles when only one entity is selected', () => {
    const result = hitTest(inputs([t1, t2], ['t1']), { x: 100, y: 100 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', entityId: 't1' })
  })

  it('never falls back to per-entity handles when the multi-bbox cannot form', () => {
    // A group with no sized operands contributes nothing, so no bbox forms —
    // and the renderer hides per-entity handles for a batch selection, so
    // none may be hit-testable either.
    const g = group('g1', 0, 0, 50, 50)
    const result = hitTest(inputs([t1, g], ['t1', 'g1']), { x: 100, y: 100 })
    expect(result.layer).not.toBe('resize-handles')
  })
})

describe('hit-test — drawing over a page wins by normal z-order (issue #123)', () => {
  // Page at (200, 200), 400×300 → body y ∈ [200, 500]. Drawing at (220, 170),
  // 100×80 → body x ∈ [220,320], y ∈ [170,250], declared after the page (front).
  // With chrome retired and the drawing-priority hack gone, the drawing wins
  // wherever it overlaps purely because it paints in front — selected or not.
  const p = page({ id: 'p1', screenX: 200, screenY: 200 })
  const d = drawing('d1', 220, 170, 100, 80)

  it('drawing over the page top edge wins (drag-from-old-chrome-area)', () => {
    // y=185 is above the page body but inside the drawing body.
    const result = hitTest(inputs([p, d]), { x: 260, y: 185 })
    expect(result.layer).toBe('body')
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 'd1', entityKind: 'drawing' })
  })

  it('drawing beats page body where they overlap', () => {
    // y=215 is inside both the page body and the drawing body.
    const result = hitTest(inputs([p, d]), { x: 260, y: 215 })
    expect(result.layer).toBe('body')
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 'd1', entityKind: 'drawing' })
  })

  it('the win does not depend on selection (front z-order alone)', () => {
    const result = hitTest(inputs([p, d], []), { x: 260, y: 185 })
    expect(result.payload).toMatchObject({ kind: 'entity-body', entityId: 'd1' })
  })

  it('resize handles of the selected drawing still win over drawing body', () => {
    // SE resize handle of drawing: corner at (320+2, 250+2) = (322, 252).
    const result = hitTest(inputs([p, d], ['d1']), { x: 322, y: 252 })
    expect(result.layer).toBe('resize-handles')
    expect(result.payload).toMatchObject({ kind: 'resize-handle', entityId: 'd1' })
  })

  it('the drawing does not leak into areas it does not cover', () => {
    // y=350 is inside the page body only, not the drawing. Should return page-body.
    const result = hitTest(inputs([p, d]), { x: 260, y: 350 })
    expect(result.layer).toBe('body')
    expect(result.payload).toMatchObject({ kind: 'page-body', entityId: 'p1' })
  })
})

describe('hit-test — auto-layout reorder dots (ADR 0015)', () => {
  // Managed-row group g1 with two text children c1, c2. Child c1 at screen
  // (220,220) 100×40 → center (270,240); reorder hit square is 14px centered →
  // x ∈ [263,277], y ∈ [233,247].
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

  it('the reorder square shrinks with the child so a zoomed-out body stays draggable', () => {
    // Zoomed way out: c1 is 20×8 on screen. A fixed 28px handle would swallow
    // the whole child, making group-drag unreachable; the capped one leaves the
    // body grabbable 3px off center.
    const tiny = { ...text('c1', 220, 220, 20, 8), canvasX: 220, canvasY: 220 }
    const tiny2 = { ...text('c2', 260, 220, 20, 8), canvasX: 260, canvasY: 220 }
    const scene = [managedGroup(['c1', 'c2']), tiny, tiny2]
    const groupSelected = {
      entities: scene,
      edges: [],
      selectedEntityIds: [],
      selectedGroupId: 'g1',
      zoom: 1,
    }
    expect(hitTest(groupSelected, { x: 230, y: 224 }).layer).toBe('reorder-handle')
    // 8px tall → hit square is 8*0.4 = 3.2px, so ±3px off center is body.
    const offCenter = hitTest(groupSelected, { x: 233, y: 224 })
    expect(offCenter.payload.kind).not.toBe('reorder-handle')
    expect(offCenter.payload).toMatchObject({ entityId: 'c1' })
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

describe('hit-test, group labels', () => {
  // Group at (100,100). Label box: 20.5px tall above the top edge, width from
  // groupLabelWidths (or the estimate fallback). Fixture label is 'g'.
  const g = group('g1', 100, 100, 600, 500)
  const widths = new Map([['g1', 40]])

  it('a point in the label box routes to group-label', () => {
    const result = hitTest(inputs([g], [], { groupLabelWidths: widths }), { x: 120, y: 90 })
    expect(result.layer).toBe('group-label')
    expect(result.payload).toEqual({ kind: 'group-label', groupId: 'g1' })
  })

  it('a point past the measured label width misses the label', () => {
    const result = hitTest(inputs([g], [], { groupLabelWidths: widths }), { x: 150, y: 90 })
    expect(result.payload.kind).not.toBe('group-label')
  })

  it('the label wins over the selected group\'s n resize handle where they overlap', () => {
    // n-handle strip is centered on the padded top edge; the label's lower
    // pixels overlap it. The small text target must stay grabbable.
    const result = hitTest(inputs([g], ['g1'], { groupLabelWidths: widths }), { x: 120, y: 97 })
    expect(result.layer).toBe('group-label')
  })

  it('falls back to an estimated width when no measurement is provided', () => {
    const result = hitTest(inputs([g]), { x: 103, y: 90 })
    expect(result.payload).toEqual({ kind: 'group-label', groupId: 'g1' })
  })

  it('an unlabeled group exposes no label target', () => {
    const bare = { ...group('g2', 100, 100, 600, 500), label: '' }
    const result = hitTest(inputs([bare], [], { groupLabelWidths: widths }), { x: 120, y: 90 })
    expect(result.payload.kind).not.toBe('group-label')
  })
})
