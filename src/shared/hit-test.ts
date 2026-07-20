/**
 * Hit-tester for canvas pointer events arriving from aboveView.
 *
 * Replaces the per-layer onMouseDown / DOM-stacking arbitration in bgView.
 * See docs/adr/0001-click-to-enter-page-focus.md.
 *
 * Pure: no Electron, no DOM. Selectors derive screen-space HitTargets from
 * the canvas scene; hitTest walks them by priority and returns the winner.
 *
 * Lives in src/shared so the renderer canvas-pointer-router and main-side
 * test routes can share one implementation without an IPC roundtrip.
 */

import { regionContains, type HitRegion, type Point, type Rect } from './hit-regions'
import {
  EDGE_ANCHOR_HIT_ACROSS_PX,
  EDGE_ANCHOR_HIT_ALONG_PX,
  EDGE_ANCHOR_HIT_GAP_PX,
  EDGE_SIDES,
  MULTI_SELECTION_OUTLINE_PADDING_PX,
  REORDER_HANDLE_HIT_PX,
  RESIZE_HANDLE_HIT_PX,
  scaleEdgeAnchorHitSize,
} from './canvas-hit-geometry'
import type {
  CanvasEntityKind,
  CanvasSceneEntity,
  EdgeSide,
  WorkspaceEdge,
} from './types'
import { HIT_LAYER_ORDER, type HitLayer } from './interaction-priority'
import { estimateGroupLabelWidth, groupLabelRect } from './group-label-geometry'
import { reorderableDots } from './reorderable-dots'
import { collectGapHandleZones } from './gap-handles'
import { ENTITY_KIND_CAPS } from './entity-kind-caps'
import { selectionBbox, type SelectionBbox } from './selection-bbox'

// --- Public types ---

export type { ResizeHandle } from './resize-accumulator'
import type { ResizeHandle } from './resize-accumulator'

export type HitPayload =
  | { kind: 'resize-handle'; entityId: string; entityKind: CanvasEntityKind; handle: ResizeHandle }
  | { kind: 'multi-resize-handle'; handle: ResizeHandle }
  | { kind: 'anchor'; entityId: string; entityKind: CanvasEntityKind; side: EdgeSide }
  | { kind: 'reorder-handle'; entityId: string; entityKind: CanvasEntityKind }
  | { kind: 'gap-handle'; groupId: string | null }
  | { kind: 'page-body'; entityId: string }
  | { kind: 'group-border'; groupId: string }
  | { kind: 'group-label'; groupId: string }
  | {
      kind: 'entity-body'
      entityId: string
      entityKind: CanvasEntityKind
      /** Only set for files; gates the dblclick / press-deferral → edit paths. */
      rendererEditable?: boolean
      /** Only set for files; true for live-iframe renderers (HTML) that get
       *  the select-first / interact-second treatment. */
      rendererInteractive?: boolean
    }
  | { kind: 'background' }

export interface HitTarget {
  layer: HitLayer
  region: HitRegion
  payload: HitPayload
}

export interface HitInputs {
  entities: readonly CanvasSceneEntity[]
  edges: readonly WorkspaceEdge[]
  selectedEntityIds: readonly string[]
  selectedGroupId?: string | null
  /** Optional. When set, anchor dots on the hovered entity are routable too —
   *  matches the EdgeLayer renderer policy (selected + hovered show anchors)
   *  and lets users grab an existing edge endpoint without first selecting
   *  the connected node. */
  hoveredEntityId?: string | null
  zoom: number
  /** Measured group-label text widths (renderer canvas measureText), keyed by
   *  group id. Optional — absent widths fall back to a per-char estimate so
   *  main-side test routes stay usable without a DOM. */
  groupLabelWidths?: ReadonlyMap<string, number>
}

// --- Top-level hit-test ---

const BACKGROUND_TARGET: HitTarget = {
  layer: 'background',
  region: { kind: 'rect', rect: { x: -Infinity, y: -Infinity, width: Infinity, height: Infinity } },
  payload: { kind: 'background' },
}

export function hitTest(inputs: HitInputs, point: Point): HitTarget {
  for (const layer of HIT_LAYER_ORDER) {
    const targets = collectLayerTargets(layer, inputs)
    // First registered match wins within a layer. Selectors are responsible
    // for ordering within a layer (e.g. front-to-back z-order for entities).
    for (const target of targets) {
      if (regionContains(target.region, point)) return target
    }
  }
  return BACKGROUND_TARGET
}

// --- Layer collectors ---

function collectLayerTargets(layer: HitLayer, inputs: HitInputs): HitTarget[] {
  switch (layer) {
    case 'group-label':
      return collectGroupLabelTargets(inputs)
    case 'resize-handles':
      return collectResizeHandles(inputs)
    case 'anchors':
      return collectAnchorTargets(inputs)
    case 'reorder-handle':
      return collectReorderHandleTargets(inputs)
    case 'gap-handle':
      return collectGapHandleTargets(inputs)
    case 'body':
      return collectBodyTargets(inputs)
    case 'background':
      return []
  }
}

// --- Selectors ---

// Group titles — the grabbable text above each group's top-left corner.
// Geometry comes from the shared `groupLabelRect` the canvas painter also
// uses, so the visible text and the routable target line up by construction.
function collectGroupLabelTargets(inputs: HitInputs): HitTarget[] {
  const out: HitTarget[] = []
  for (const entity of inputs.entities) {
    if (entity.kind !== 'group' || !entity.label) continue
    const width =
      inputs.groupLabelWidths?.get(entity.id) ?? estimateGroupLabelWidth(entity.label)
    out.push({
      layer: 'group-label',
      region: { kind: 'rect', rect: groupLabelRect(entity, width) },
      payload: { kind: 'group-label', groupId: entity.id },
    })
  }
  return out
}

function collectResizeHandles(inputs: HitInputs): HitTarget[] {
  const out: HitTarget[] = []
  const selectedIds = new Set(inputs.selectedEntityIds)
  const multiSelectionContainsGroup =
    inputs.selectedEntityIds.length > 1 &&
    inputs.entities.some((entity) => entity.kind === 'group' && selectedIds.has(entity.id))
  if (multiSelectionContainsGroup) return out

  // Multi-selection: per-entity handles are visually hidden in favor of one
  // bbox spanning the selection. Mirror that here — emit the eight multi-bbox
  // handles plus per-entity handles for any selected group (groups have
  // their own selection overlay independent of the multi-box). Fall through
  // to the single-entity path if a bbox can't be formed (e.g. fewer than two
  // non-group entities once groups are excluded).
  const bbox =
    inputs.selectedEntityIds.length > 1
      ? selectionBbox(inputs.entities, inputs.selectedEntityIds, 'screen')
      : null
  if (bbox) {
    for (const handle of HANDLES) {
      out.push({
        layer: 'resize-handles',
        region: { kind: 'rect', rect: multiHandleRect(bbox, handle) },
        payload: { kind: 'multi-resize-handle', handle },
      })
    }
    if (inputs.selectedGroupId) {
      const group = inputs.entities.find((e) => e.id === inputs.selectedGroupId)
      if (group) pushPerEntityHandles(out, group)
    }
    return out
  }

  const selected = new Set(inputs.selectedEntityIds)
  if (inputs.selectedGroupId) selected.add(inputs.selectedGroupId)
  for (const entity of inputs.entities) {
    if (!selected.has(entity.id)) continue
    pushPerEntityHandles(out, entity)
  }
  return out
}

function pushPerEntityHandles(out: HitTarget[], entity: CanvasSceneEntity): void {
  for (const handle of HANDLES) {
    out.push({
      layer: 'resize-handles',
      region: { kind: 'rect', rect: handleRect(entity, handle) },
      payload: {
        kind: 'resize-handle',
        entityId: entity.id,
        entityKind: entity.kind,
        handle,
      },
    })
  }
}

function multiHandleRect(bbox: SelectionBbox, handle: ResizeHandle): Rect {
  const half = RESIZE_HANDLE_HIT_PX / 2
  const pad = MULTI_SELECTION_OUTLINE_PADDING_PX
  const left = bbox.x - pad
  const top = bbox.y - pad
  const right = bbox.x + bbox.width + pad
  const bottom = bbox.y + bbox.height + pad
  switch (handle) {
    case 'nw':
      return { x: left - half, y: top - half, width: RESIZE_HANDLE_HIT_PX, height: RESIZE_HANDLE_HIT_PX }
    case 'ne':
      return { x: right - half, y: top - half, width: RESIZE_HANDLE_HIT_PX, height: RESIZE_HANDLE_HIT_PX }
    case 'se':
      return { x: right - half, y: bottom - half, width: RESIZE_HANDLE_HIT_PX, height: RESIZE_HANDLE_HIT_PX }
    case 'sw':
      return { x: left - half, y: bottom - half, width: RESIZE_HANDLE_HIT_PX, height: RESIZE_HANDLE_HIT_PX }
    case 'n':
      return { x: left, y: top - half, width: right - left, height: RESIZE_HANDLE_HIT_PX }
    case 's':
      return { x: left, y: bottom - half, width: right - left, height: RESIZE_HANDLE_HIT_PX }
    case 'w':
      return { x: left - half, y: top, width: RESIZE_HANDLE_HIT_PX, height: bottom - top }
    case 'e':
      return { x: right - half, y: top, width: RESIZE_HANDLE_HIT_PX, height: bottom - top }
  }
}

/**
 * Anchor eligibility — the one definition of which entities show (and route)
 * edge anchors, shared by the EdgeLayer painter and `collectAnchorTargets` so
 * the visible dots and the routable targets can't drift into invisible-but-
 * grabbable anchors. Edge creation is a single-node affordance: a
 * multi-selection suppresses all anchors (so a hidden anchor over the gap
 * handle can't hijack the gap-spacing drag). Hover stays eligible so a user
 * can grab an existing edge endpoint to re-route or delete without first
 * selecting the node; `edgeSelected` suppresses the selection's own anchors
 * while an edge is selected (hover only).
 */
export function anchorEligibleEntityIds(input: {
  selectedEntityIds: readonly string[]
  selectedGroupId?: string | null
  hoveredEntityId?: string | null
  edgeSelected?: boolean
}): Set<string> {
  const eligible = new Set<string>()
  if (input.selectedEntityIds.length > 1) return eligible
  if (!input.edgeSelected) {
    for (const id of input.selectedEntityIds) eligible.add(id)
    if (input.selectedGroupId) eligible.add(input.selectedGroupId)
  }
  if (input.hoveredEntityId) eligible.add(input.hoveredEntityId)
  return eligible
}

function collectAnchorTargets(inputs: HitInputs): HitTarget[] {
  const eligible = anchorEligibleEntityIds(inputs)
  if (eligible.size === 0) return []
  const out: HitTarget[] = []
  for (const entity of inputs.entities) {
    if (!entityHasAnchors(entity.kind)) continue
    if (!eligible.has(entity.id)) continue
    for (const side of EDGE_SIDES) {
      out.push({
        layer: 'anchors',
        region: { kind: 'rect', rect: anchorRect(entity, side, inputs.zoom) },
        payload: { kind: 'anchor', entityId: entity.id, entityKind: entity.kind, side },
      })
    }
  }
  return out
}

// Center-dot reorder handles. Eligibility is the union of both reorder doors —
// loose equal-gap selection plus a selected managed-row group's children — via
// the one shared `reorderableDots` selector (ADR 0015 D7). The dot's screen
// center comes straight from the selector, so the visible dot and the grabbable
// target line up by construction. The begin payload carries only the entity id;
// main resolves which door armed the gesture.
function collectReorderHandleTargets(inputs: HitInputs): HitTarget[] {
  return reorderableDots(inputs).map((dot) => ({
    layer: 'reorder-handle' as const,
    region: { kind: 'rect' as const, rect: reorderHandleRectAt(dot.center) },
    payload: { kind: 'reorder-handle' as const, entityId: dot.id, entityKind: dot.entityKind },
  }))
}

// Gap strips between a managed group's adjacent children (ADR 0015 Milestone
// 2). Geometry and eligibility come from the one shared `collectGapHandleZones`
// selector — the same source `GapHandlesLayer` paints — so the visible strip
// and the grabbable target line up by construction.
function collectGapHandleTargets(inputs: HitInputs): HitTarget[] {
  return collectGapHandleZones(inputs).map((zone) => ({
    layer: 'gap-handle' as const,
    region: { kind: 'rect' as const, rect: zone.rect },
    payload: { kind: 'gap-handle' as const, groupId: zone.groupId },
  }))
}

function collectBodyTargets(inputs: HitInputs): HitTarget[] {
  // Mirror the physical paint planes first, then entityOrder within each
  // plane. Notes (all non-page bodies) live in aboveView and therefore always
  // paint above page WCVs, even when the flat persisted entityOrder ranks a
  // page later. Within Notes and within Pages, reverse the back-to-front input
  // order so the front-most sibling wins. Groups stay last because they're
  // containers — members painted inside them must hit first.
  const groups: HitTarget[] = []
  const notes: HitTarget[] = []
  const pages: HitTarget[] = []
  for (let i = inputs.entities.length - 1; i >= 0; i--) {
    const entity = inputs.entities[i]
    const target: HitTarget = {
      layer: 'body',
      region: { kind: 'rect', rect: bodyRect(entity) },
      payload:
        entity.kind === 'page'
          ? { kind: 'page-body', entityId: entity.id }
          : {
              kind: 'entity-body',
              entityId: entity.id,
              entityKind: entity.kind,
              rendererEditable:
                entity.kind === 'file' ? entity.rendererEditable === true : undefined,
              rendererInteractive:
                entity.kind === 'file' ? entity.rendererInteractive === true : undefined,
            },
    }
    if (entity.kind === 'group') {
      const border = 8
      const { screenX: x, screenY: y, screenWidth: width, screenHeight: height } = entity
      const borderRects: Rect[] = [
        { x, y, width, height: border },
        { x, y: y + height - border, width, height: border },
        { x, y: y + border, width: border, height: Math.max(0, height - border * 2) },
        { x: x + width - border, y: y + border, width: border, height: Math.max(0, height - border * 2) },
      ]
      groups.push(
        ...borderRects.map((rect) => ({
          layer: 'body' as const,
          region: { kind: 'rect' as const, rect },
          payload: { kind: 'group-border' as const, groupId: entity.id },
        })),
        target,
      )
    }
    else if (entity.kind === 'page') pages.push(target)
    else notes.push(target)
  }
  return [...notes, ...pages, ...groups]
}

// --- Geometry helpers ---

// Order matters within a layer — first registered match wins. Corners come
// before edges so a click at the very corner of a wide entity routes to the
// corner handle (diagonal resize) rather than the edge strip that runs
// through it.
const HANDLES: readonly ResizeHandle[] = ['nw', 'ne', 'se', 'sw', 'n', 'e', 's', 'w']

// Selection outlines sit slightly outside the entity body; resize handles
// are centered on the outline corners/edges, not the entity itself. Match
// the padding used by SelectionOutlineLayer so hit-test geometry tracks the
// pixels users actually see.
const SINGLE_SELECTION_OUTLINE_PADDING_PX = 2

function outlinePaddingFor(_kind: CanvasEntityKind): number {
  return SINGLE_SELECTION_OUTLINE_PADDING_PX
}

function handleRect(entity: CanvasSceneEntity, handle: ResizeHandle): Rect {
  const half = RESIZE_HANDLE_HIT_PX / 2
  const pad = outlinePaddingFor(entity.kind)
  const { screenX: x, screenY: y, screenWidth: w, screenHeight: h } = entity
  switch (handle) {
    case 'nw':
      return { x: x - pad - half, y: y - pad - half, width: RESIZE_HANDLE_HIT_PX, height: RESIZE_HANDLE_HIT_PX }
    case 'ne':
      return { x: x + w + pad - half, y: y - pad - half, width: RESIZE_HANDLE_HIT_PX, height: RESIZE_HANDLE_HIT_PX }
    case 'se':
      return { x: x + w + pad - half, y: y + h + pad - half, width: RESIZE_HANDLE_HIT_PX, height: RESIZE_HANDLE_HIT_PX }
    case 'sw':
      return { x: x - pad - half, y: y + h + pad - half, width: RESIZE_HANDLE_HIT_PX, height: RESIZE_HANDLE_HIT_PX }
    // Edge handles run the full length of the entity edge — visually they
    // span corner-to-corner. Corners are checked first (HANDLES order), so
    // a click at the very corner still resolves to nw/ne/sw/se.
    case 'n':
      return { x, y: y - pad - half, width: w, height: RESIZE_HANDLE_HIT_PX }
    case 's':
      return { x, y: y + h + pad - half, width: w, height: RESIZE_HANDLE_HIT_PX }
    case 'w':
      return { x: x - pad - half, y, width: RESIZE_HANDLE_HIT_PX, height: h }
    case 'e':
      return { x: x + w + pad - half, y, width: RESIZE_HANDLE_HIT_PX, height: h }
  }
}

function reorderHandleRectAt(center: { x: number; y: number }): Rect {
  const half = REORDER_HANDLE_HIT_PX / 2
  return { x: center.x - half, y: center.y - half, width: REORDER_HANDLE_HIT_PX, height: REORDER_HANDLE_HIT_PX }
}

function bodyRect(entity: CanvasSceneEntity): Rect {
  return {
    x: entity.screenX,
    y: entity.screenY,
    width: entity.screenWidth,
    height: entity.screenHeight,
  }
}

function anchorRect(entity: CanvasSceneEntity, side: EdgeSide, zoom: number): Rect {
  const along = scaleEdgeAnchorHitSize(EDGE_ANCHOR_HIT_ALONG_PX, zoom)
  const across = scaleEdgeAnchorHitSize(EDGE_ANCHOR_HIT_ACROSS_PX, zoom)
  const horizontal = side === 'top' || side === 'bottom'
  const w = horizontal ? along : across
  const h = horizontal ? across : along
  const cx = entity.screenX + entity.screenWidth / 2
  const cy = entity.screenY + entity.screenHeight / 2
  switch (side) {
    case 'top':
      return { x: cx - w / 2, y: entity.screenY - EDGE_ANCHOR_HIT_GAP_PX - h, width: w, height: h }
    case 'bottom':
      return { x: cx - w / 2, y: entity.screenY + entity.screenHeight + EDGE_ANCHOR_HIT_GAP_PX, width: w, height: h }
    case 'left':
      return { x: entity.screenX - EDGE_ANCHOR_HIT_GAP_PX - w, y: cy - h / 2, width: w, height: h }
    case 'right':
      return { x: entity.screenX + entity.screenWidth + EDGE_ANCHOR_HIT_GAP_PX, y: cy - h / 2, width: w, height: h }
  }
}

export function entityHasAnchors(kind: CanvasEntityKind): boolean {
  return ENTITY_KIND_CAPS[kind].hasAnchors
}
