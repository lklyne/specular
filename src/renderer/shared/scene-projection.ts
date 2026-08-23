/**
 * The renderer's projection boundary: canvas-space scene entities in, screen
 * geometry out, computed here rather than read off the wire.
 *
 * Every layer downstream of this file places DOM at coordinates this renderer
 * derived from the camera it holds, so a layer's position is a function of one
 * value instead of a race between two. `projectSceneEntities` is where a scene
 * from main crosses that line; `reprojectEntity` is where a locally-derived
 * entity (a drag ghost, a measured sticky, a reorder preview) crosses it.
 *
 * Identity is load-bearing: the memoized layers bail out on reference equality,
 * so a projection that has not changed must hand back the object it handed back
 * last time. Both caches key on the source object and compare the camera by
 * value, because a caller that rebuilds `{zoom, pan}` per call is normal.
 */

import {
  projectSceneEntity,
  type SceneCamera,
  type ScenePoint,
} from '../../shared/scene-projection'
import type { CanvasSceneEntity, LayoutUpdateData } from '../../shared/types'
import { driftWatchdogEnabled } from './runtime-store-drift'

interface ProjectionKey {
  zoom: number
  panX: number
  panY: number
  originX: number
  originY: number
}

function projectionKey(camera: SceneCamera, sceneOrigin: ScenePoint): ProjectionKey {
  return {
    zoom: camera.zoom,
    panX: camera.pan.x,
    panY: camera.pan.y,
    originX: sceneOrigin.x,
    originY: sceneOrigin.y,
  }
}

function sameKey(a: ProjectionKey, b: ProjectionKey): boolean {
  return (
    a.zoom === b.zoom &&
    a.panX === b.panX &&
    a.panY === b.panY &&
    a.originX === b.originX &&
    a.originY === b.originY
  )
}

const entityCache = new WeakMap<object, { key: ProjectionKey; value: CanvasSceneEntity }>()
const listCache = new WeakMap<object, { key: ProjectionKey; value: CanvasSceneEntity[] }>()

/** One entity's screen geometry, recomputed from the camera. */
export function projectEntity<T extends CanvasSceneEntity>(
  entity: T,
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): T {
  const key = projectionKey(camera, sceneOrigin)
  const cached = entityCache.get(entity)
  if (cached && sameKey(cached.key, key)) return cached.value as T
  const value = { ...entity, ...projectSceneEntity(entity, camera, sceneOrigin) } as T
  entityCache.set(entity, { key, value })
  return value
}

/**
 * An entity the renderer derived rather than received — its canvas geometry has
 * already moved, so its screen geometry has to be recomputed rather than nudged
 * by hand alongside it.
 */
export function reprojectEntity<T extends CanvasSceneEntity>(
  entity: T,
  layout: Pick<LayoutUpdateData, 'zoom' | 'pan' | 'canvasOrigin'>,
): T {
  return projectEntity(entity, { zoom: layout.zoom, pan: layout.pan }, layout.canvasOrigin)
}

/** A scene from main, projected. The array keeps its identity when nothing moved. */
export function projectSceneEntities<T extends CanvasSceneEntity>(
  entities: readonly T[],
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): T[] {
  const key = projectionKey(camera, sceneOrigin)
  const cached = listCache.get(entities)
  if (cached && sameKey(cached.key, key)) return cached.value as T[]
  const value = entities.map((entity) => {
    const projected = projectEntity(entity, camera, sceneOrigin)
    reportProjectionDrift(entity, projected)
    return projected
  })
  listCache.set(entities, { key, value })
  return value
}

const layoutCache = new WeakMap<object, LayoutUpdateData>()

/** The whole payload with its scene projected, for the layers that still read
 *  `layoutData.entities` and `layoutData.groups` rather than a slice. */
export function projectLayoutData(layout: LayoutUpdateData): LayoutUpdateData {
  const cached = layoutCache.get(layout)
  if (cached) return cached
  const camera = { zoom: layout.zoom, pan: layout.pan }
  const entities = projectSceneEntities(layout.entities, camera, layout.canvasOrigin)
  const groups = layout.groups
    ? projectSceneEntities(layout.groups, camera, layout.canvasOrigin)
    : layout.groups
  const value = { ...layout, entities, groups }
  layoutCache.set(layout, value)
  return value
}

// --- drift assertion -------------------------------------------------------

/**
 * While main still stamps `screen*` onto every entity, the two projections are
 * checkable against each other. A disagreement is a projection bug, and it is
 * silent otherwise: the DOM simply sits a few pixels off the native page view.
 *
 * `console.warn` lands in `errors.log` via `wireRendererLogging`, so a stress
 * session's drift is readable after the fact. One line per (kind, field) — a
 * wrong formula is one bug, not one per entity per frame.
 */
const DRIFT_TOLERANCE_PX = 0.5
const SCREEN_FIELDS = [
  'screenX',
  'screenY',
  'screenWidth',
  'screenHeight',
  'contentScreenX',
  'contentScreenY',
  'contentScreenWidth',
  'contentScreenHeight',
] as const

const reportedDrift = new Set<string>()

function reportProjectionDrift(sent: CanvasSceneEntity, local: CanvasSceneEntity): void {
  if (!driftWatchdogEnabled) return
  const from = sent as unknown as Record<string, number | undefined>
  const to = local as unknown as Record<string, number | undefined>
  for (const field of SCREEN_FIELDS) {
    const main = from[field]
    const projected = to[field]
    if (main === undefined && projected === undefined) continue
    if (
      main !== undefined &&
      projected !== undefined &&
      Math.abs(main - projected) <= DRIFT_TOLERANCE_PX
    ) {
      continue
    }
    const cell = `${sent.kind}.${field}`
    if (reportedDrift.has(cell)) continue
    reportedDrift.add(cell)
    console.warn(
      `[projection-drift] ${cell} main=${main} local=${projected} (entity ${sent.id})`,
    )
  }
}
