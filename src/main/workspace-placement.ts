import type {
  ApplyDirectiveRequest,
  ApplyDirectiveResult,
  BatchPlacementRequest,
  BatchPlacementResult,
  PlacementRequest,
  PlacementResult,
  WorkspaceBounds,
  WorkspacePage,
} from '../shared/types'
import { resolveSpacing } from '../shared/layout-directive'
import {
  ANCHOR_OFFSET_X,
  ANCHOR_OFFSET_Y,
  CLUSTER_HORIZONTAL_GUTTER,
  CLUSTER_OUTER_MARGIN,
  PLACEMENT_SCAN_STEP,
} from '../shared/constants'
import { pages } from './runtime/page-runtime'
import { textEntities } from './runtime/text-entity-state'
import { fileEntities } from './runtime/file-entity-state'
import { snapToGrid } from '../shared/gesture-utils'
import { workspaceGroups } from './runtime/workspace-model'
import { boundsOverlap, pageSnapBounds } from './runtime/runtime-geometry'
import { CHROME_HEADER_HEIGHT } from './runtime/runtime-constants'
import {
  allWorkspacePages,
  entityBoundsById,
  entityDataInsetsById,
  entityKindById,
  selectionBounds,
  unionBounds,
} from './workspace-entities'
import { computeLayoutMetrics, computeLayoutPositions, type LayoutBox } from './layout-math'

// Chrome headers render above each entity's canvasY (see EntityChromeHeader —
// translateY(-100%)). Extend the occupied rect upward so placement treats the
// header band as claimed too. Without this, a new entity's body can land
// flush against the bottom of an entity above, with its own chrome header
// intruding into the visible gutter.
function extendUpwardForChrome(bounds: WorkspaceBounds, headerHeight: number): WorkspaceBounds {
  return {
    x: bounds.x,
    y: bounds.y - headerHeight,
    width: bounds.width,
    height: bounds.height + headerHeight,
  }
}

export function occupiedRegions(): WorkspaceBounds[] {
  return [
    // Pages: use the snap rect (body + device-frame insets) and extend
    // upward by the chrome strip.
    ...pages.map((page) =>
      extendUpwardForChrome(pageSnapBounds(page), CHROME_HEADER_HEIGHT),
    ),
    ...textEntities.map((entity) => ({
      x: entity.canvasX,
      y: entity.canvasY,
      width: entity.width,
      height: entity.height,
    })),
    // File entities may render a chrome header (markdown/wireframe). Claim the
    // header band unconditionally — a small over-claim on plain files is fine.
    ...fileEntities.map((entity) =>
      extendUpwardForChrome(
        { x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height },
        CHROME_HEADER_HEIGHT,
      ),
    ),
    ...workspaceGroups.map((group) => ({
      x: group.canvasX,
      y: group.canvasY,
      width: group.width,
      height: group.height,
    })),
  ]
}

export function expandBounds(bounds: WorkspaceBounds, padding: number): WorkspaceBounds {
  return {
    x: bounds.x - padding,
    y: bounds.y - padding,
    width: bounds.width + padding * 2,
    height: bounds.height + padding * 2,
  }
}

function candidateCollides(
  candidate: WorkspaceBounds,
  regions: WorkspaceBounds[] = occupiedRegions(),
): boolean {
  // Reserve a chrome-header band above the candidate body. We don't know the
  // kind of the entity being placed here, so assume it may have a header —
  // worst case this leaves 44px of extra headroom for a headerless entity.
  const inflated = extendUpwardForChrome(candidate, CHROME_HEADER_HEIGHT)
  return regions.some((bounds) =>
    boundsOverlap(inflated, expandBounds(bounds, CLUSTER_OUTER_MARGIN)),
  )
}

function scanForPlacement(
  width: number,
  height: number,
  startX: number,
  startY: number,
): PlacementResult {
  const pages = allWorkspacePages()
  const maxRight = Math.max(
    2000,
    ...pages.map((page) => page.canvasX + page.width + CLUSTER_OUTER_MARGIN),
  )
  const maxBottom = Math.max(
    2000,
    ...pages.map((page) => page.canvasY + page.height + CLUSTER_OUTER_MARGIN),
  )
  const limitX = maxRight + width + 2000
  const limitY = maxBottom + height + 2000
  const regions = occupiedRegions()

  for (let y = snapToGrid(startY); y <= limitY; y += PLACEMENT_SCAN_STEP) {
    const rowStartX = y === snapToGrid(startY) ? snapToGrid(startX) : CLUSTER_OUTER_MARGIN
    for (let x = rowStartX; x <= limitX; x += PLACEMENT_SCAN_STEP) {
      const candidate = { x, y, width, height }
      if (!candidateCollides(candidate, regions)) {
        return {
          canvasX: x,
          canvasY: y,
          fallbackUsed: x !== snapToGrid(startX) || y !== snapToGrid(startY),
          reason: 'scan_fit',
        }
      }
    }
  }

  throw new Error('No legal placement found within scan bounds')
}

export function findPlacement(request: PlacementRequest): PlacementResult {
  const width = snapToGrid(request.width)
  const height = snapToGrid(request.height)

  if (request.anchor === 'selection_or_empty_region') {
    const anchor = selectionBounds()
    if (anchor) {
      const initialX = snapToGrid(anchor.x + anchor.width + ANCHOR_OFFSET_X)
      const initialY = snapToGrid(anchor.y + ANCHOR_OFFSET_Y)
      const candidate = { x: initialX, y: initialY, width, height }
      if (!candidateCollides(candidate)) {
        return {
          canvasX: initialX,
          canvasY: initialY,
          fallbackUsed: false,
          reason: 'selection_anchor',
        }
      }
      return scanForPlacement(width, height, initialX, initialY)
    }
  }

  return scanForPlacement(width, height, CLUSTER_OUTER_MARGIN, CLUSTER_OUTER_MARGIN)
}

/**
 * Find non-overlapping positions for a batch of items, laid out as a group.
 * Computes the bounding box for the entire batch, calls findPlacement() once,
 * then positions children sequentially with uniform gaps.
 */
export function findBatchPlacement(request: BatchPlacementRequest): BatchPlacementResult {
  const gap = snapToGrid(request.gap ?? CLUSTER_HORIZONTAL_GUTTER)
  const layout = request.layout ?? 'row'
  const items = request.items.map((i) => ({
    width: snapToGrid(i.width),
    height: snapToGrid(i.height),
  }))
  const insets = request.items.map((i) => ({
    insetX: i.insetX ?? 0,
    insetY: i.insetY ?? 0,
  }))

  if (items.length === 0) return { positions: [] }

  const metrics = computeLayoutMetrics(items, layout, gap, gap)
  const placement = findPlacement({
    width: metrics.bbWidth,
    height: metrics.bbHeight,
    anchor: request.anchor ?? 'selection_or_empty_region',
  })

  const outerPositions = computeLayoutPositions(
    items,
    layout,
    gap,
    gap,
    { x: placement.canvasX, y: placement.canvasY },
    metrics.cols,
  )

  const positions = outerPositions.map((p, idx) => ({
    canvasX: p.canvasX + insets[idx].insetX,
    canvasY: p.canvasY + insets[idx].insetY,
  }))

  return { positions }
}

/**
 * Apply a layout directive: resolve origin (originX/Y → near → bbox of
 * existing items → findPlacement), look up sizes for items carrying an `id`,
 * compute positions. Used by upsertEntities when a `layout` directive is
 * present.
 *
 * Layout runs in OUTER space so `gap` measures visible whitespace between
 * device-shell bezels; positions are offset back to inner data-origin
 * coordinates before returning. User-supplied origins are honored exactly;
 * the no-anchor fallback uses `findPlacement`, which still snaps to grid.
 */
export function applyLayoutDirective(request: ApplyDirectiveRequest): ApplyDirectiveResult {
  const directive = request.layout
  const kind = directive.kind
  const baseGap = resolveSpacing(directive.gap, CLUSTER_HORIZONTAL_GUTTER)
  const colGap = resolveSpacing(directive.colGap, baseGap)
  const rowGap = resolveSpacing(directive.rowGap, baseGap)
  const warnings: string[] = []

  // Single pass: collect everything we need per item — outer footprint, data
  // insets, kind, and (for re-layouts) the existing entity's bounds for the
  // implicit-origin fallback. Avoids re-traversing the entity graph 3× per id.
  const items: LayoutBox[] = []
  const itemInsets: Array<{ insetX: number; insetY: number }> = []
  const kinds: ApplyDirectiveResult['kinds'] = []
  const existingBounds: WorkspaceBounds[] = []
  for (let idx = 0; idx < request.items.length; idx++) {
    const it = request.items[idx]
    if (it.id) {
      const bounds = entityBoundsById(it.id)
      if (!bounds) {
        throw new Error(`applyLayoutDirective: unknown entity id "${it.id}" at index ${idx}`)
      }
      items.push({ width: it.width ?? bounds.width, height: it.height ?? bounds.height })
      itemInsets.push(entityDataInsetsById(it.id))
      kinds.push(entityKindById(it.id))
      existingBounds.push(bounds)
      continue
    }
    if (it.width === undefined || it.height === undefined) {
      throw new Error(`applyLayoutDirective: item at index ${idx} has no id and no width/height`)
    }
    items.push({ width: it.width, height: it.height })
    itemInsets.push({ insetX: it.insetX ?? 0, insetY: it.insetY ?? 0 })
    kinds.push(null)
  }

  if (items.length === 0) return { positions: [], kinds: [] }

  // Resolve origin in OUTER space.
  let origin: { x: number; y: number }
  if (directive.originX !== undefined && directive.originY !== undefined) {
    // User specifies the first item's INNER (data-origin) position; convert
    // to outer by subtracting that item's insets so layout math is consistent.
    origin = {
      x: directive.originX - itemInsets[0].insetX,
      y: directive.originY - itemInsets[0].insetY,
    }
  } else if (directive.near) {
    const near = entityBoundsById(directive.near)
    if (!near) {
      throw new Error(`applyLayoutDirective: near entity "${directive.near}" not found`)
    }
    if (kind === 'column') {
      origin = { x: near.x, y: near.y + near.height + rowGap }
    } else {
      origin = { x: near.x + near.width + colGap, y: near.y }
    }
  } else if (existingBounds.length > 0) {
    const bbox = unionBounds(existingBounds)!
    origin = { x: bbox.x, y: bbox.y }
  } else {
    const metrics = computeLayoutMetrics(items, kind, colGap, rowGap, directive.cols)
    const placement = findPlacement({
      width: metrics.bbWidth,
      height: metrics.bbHeight,
      anchor: 'selection_or_empty_region',
    })
    origin = { x: placement.canvasX, y: placement.canvasY }
  }

  const outerPositions = computeLayoutPositions(items, kind, colGap, rowGap, origin, directive.cols)
  const positions = outerPositions.map((p, idx) => ({
    canvasX: p.canvasX + itemInsets[idx].insetX,
    canvasY: p.canvasY + itemInsets[idx].insetY,
  }))
  return warnings.length > 0 ? { positions, kinds, warnings } : { positions, kinds }
}

/**
 * Find a non-overlapping position for a duplicated entity.
 * Tries placing to the right of the source first, then below, then falls back
 * to the general scan algorithm.
 */
export function findDuplicatePlacement(source: WorkspaceBounds): { canvasX: number; canvasY: number } {
  const width = snapToGrid(source.width)
  const height = snapToGrid(source.height)
  // candidateCollides() expands occupied regions by CLUSTER_OUTER_MARGIN,
  // which is the shared visual gap between placed entities.
  const gap = CLUSTER_OUTER_MARGIN

  // 1. Try to the right of the source
  const rightX = snapToGrid(source.x + source.width + gap)
  const rightY = snapToGrid(source.y)
  const rightCandidate = { x: rightX, y: rightY, width, height }
  if (!candidateCollides(rightCandidate)) {
    return { canvasX: rightX, canvasY: rightY }
  }

  // 2. Try below the source
  const belowX = snapToGrid(source.x)
  const belowY = snapToGrid(source.y + source.height + gap)
  const belowCandidate = { x: belowX, y: belowY, width, height }
  if (!candidateCollides(belowCandidate)) {
    return { canvasX: belowX, canvasY: belowY }
  }

  // 3. Fall back to scan starting from the right candidate position
  const result = scanForPlacement(width, height, rightX, rightY)
  return { canvasX: result.canvasX, canvasY: result.canvasY }
}
