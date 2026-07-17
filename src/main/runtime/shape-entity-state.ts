import { randomUUID } from 'crypto'
import type {
  CanvasSceneShapeEntity,
  PersistedShapeEntity,
  ShapeBorderStyle,
  ShapeKind,
} from '../../shared/types'
import type { PageAnchor } from '../../shared/page-anchor'
import { markDirty } from './layout-dirty'
import { applyPatch } from './apply-patch'
import { pageAnchorScrollShift } from './page-anchor-scroll'

export interface ShapeEntity {
  id: string
  shapeKind: ShapeKind
  text: string
  color?: string
  strokeWidth?: number
  borderStyle?: ShapeBorderStyle
  borderColor?: string
  /** Per-entity text size in px for the inner label. ADR 0013 §2. */
  textSize?: number
  theme?: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  pageAnchor?: PageAnchor
  label?: string
}

// Shapes are placed as squares by default; the user resizes freely after.
export const DEFAULT_SHAPE_WIDTH = 160
export const DEFAULT_SHAPE_HEIGHT = 160
export const DEFAULT_STROKE_WIDTH = 2
export const MIN_SHAPE_WIDTH = 24
export const MIN_SHAPE_HEIGHT = 24

export function defaultShapeSize(_shapeKind: ShapeKind): { width: number; height: number } {
  return { width: DEFAULT_SHAPE_WIDTH, height: DEFAULT_SHAPE_HEIGHT }
}

export const shapeEntities: ShapeEntity[] = []

export function createShapeEntity(input: {
  canvasX: number
  canvasY: number
  shapeKind?: ShapeKind
  width?: number
  height?: number
  text?: string
  color?: string
  strokeWidth?: number
  borderStyle?: ShapeBorderStyle
  borderColor?: string
  textSize?: number
  theme?: string
  id?: string
  parentGroupId?: string
  pageAnchor?: PageAnchor
  label?: string
}): ShapeEntity {
  const shapeKind = input.shapeKind ?? 'rectangle'
  const fallback = defaultShapeSize(shapeKind)
  const entity: ShapeEntity = {
    id: input.id ?? `shape_${randomUUID()}`,
    shapeKind,
    text: input.text ?? '',
    color: input.color,
    strokeWidth: input.strokeWidth,
    borderStyle: input.borderStyle,
    borderColor: input.borderColor,
    textSize: input.textSize,
    theme: input.theme,
    canvasX: input.canvasX,
    canvasY: input.canvasY,
    width: input.width ?? fallback.width,
    height: input.height ?? fallback.height,
    parentGroupId: input.parentGroupId,
    pageAnchor: input.pageAnchor,
    label: input.label,
  }
  shapeEntities.push(entity)
  markDirty('canvas', 'sidebar')
  return entity
}

export function updateShapeEntity(
  id: string,
  patch: Partial<Omit<ShapeEntity, 'id'>>,
): ShapeEntity | null {
  const entity = shapeEntities.find((s) => s.id === id)
  if (!entity) return null
  applyPatch(entity, patch, [
    'shapeKind', 'text', 'strokeWidth', 'borderStyle', 'textSize',
    'canvasX', 'canvasY', 'width', 'height', 'parentGroupId', 'pageAnchor',
  ])
  if (patch.color !== undefined) entity.color = patch.color || undefined
  if (patch.borderColor !== undefined) entity.borderColor = patch.borderColor || undefined
  if (patch.theme !== undefined) entity.theme = patch.theme || undefined
  if (patch.label !== undefined) entity.label = patch.label || undefined
  markDirty('canvas', 'sidebar')
  return entity
}

export function deleteShapeEntity(id: string): boolean {
  const idx = shapeEntities.findIndex((s) => s.id === id)
  if (idx === -1) return false
  shapeEntities.splice(idx, 1)
  markDirty('canvas', 'sidebar')
  return true
}

export function clearShapeEntities(): void {
  shapeEntities.length = 0
}

// The field copy shared by the scene and persisted projections. Scene adds
// screen coords; persist adds `label`.
function shapeCoreFields(entity: ShapeEntity) {
  return {
    kind: 'shape' as const,
    id: entity.id,
    shapeKind: entity.shapeKind,
    text: entity.text,
    color: entity.color,
    strokeWidth: entity.strokeWidth,
    borderStyle: entity.borderStyle,
    borderColor: entity.borderColor,
    textSize: entity.textSize,
    theme: entity.theme,
    canvasX: entity.canvasX,
    canvasY: entity.canvasY,
    width: entity.width,
    height: entity.height,
    parentGroupId: entity.parentGroupId,
    pageAnchor: entity.pageAnchor,
  }
}

export function buildShapeEntitySceneEntity(
  entity: ShapeEntity,
  zoom: number,
  pan: { x: number; y: number },
  canvasOrigin: { x: number; y: number },
): CanvasSceneShapeEntity {
  // Scroll-follow: the scene projects the *apparent* position — stored canvas
  // coords shifted by how far the anchor page has scrolled since the anchor
  // was written. The stored coords stay untouched (scroll is ephemeral; see
  // rebaseAnchorScroll in page-anchor-state.ts for when they're folded).
  const shift = pageAnchorScrollShift(entity.pageAnchor)
  const canvasX = entity.canvasX - shift.x
  const canvasY = entity.canvasY - shift.y
  return {
    ...shapeCoreFields(entity),
    canvasX,
    canvasY,
    screenX: canvasOrigin.x + canvasX * zoom + pan.x,
    screenY: canvasOrigin.y + canvasY * zoom + pan.y,
    screenWidth: entity.width * zoom,
    screenHeight: entity.height * zoom,
  }
}

/**
 * Every key `persistShapeEntity` writes to the doc's entity map — the single
 * field list both sync directions derive from (ADR 0024 §5). `satisfies`
 * keeps the set exhaustive against `PersistedShapeEntity`; the persisted-fields
 * drift test keeps `persistShapeEntity` on it.
 */
const SHAPE_ENTITY_PERSISTED_FIELD_SET = {
  kind: true,
  id: true,
  shapeKind: true,
  text: true,
  color: true,
  strokeWidth: true,
  borderStyle: true,
  borderColor: true,
  textSize: true,
  theme: true,
  canvasX: true,
  canvasY: true,
  width: true,
  height: true,
  parentGroupId: true,
  pageAnchor: true,
  label: true,
} as const satisfies Record<keyof PersistedShapeEntity, true>

export const SHAPE_ENTITY_PERSISTED_FIELDS: readonly string[] = Object.keys(
  SHAPE_ENTITY_PERSISTED_FIELD_SET,
)

export function persistShapeEntity(entity: ShapeEntity): PersistedShapeEntity {
  return {
    ...shapeCoreFields(entity),
    label: entity.label,
  }
}
