/**
 * Which slices of the runtime store each canvas renderer is routed.
 *
 * A layout snapshot is one payload sent to every renderer, so the heaviest
 * thing in it is paid for three times over even though one consumer reads it —
 * `inspect`, the selected page's whole component tree, is most of the payload
 * and only agent-layer draws from it. Naming what each target reads turns that
 * into what it costs.
 *
 * The baseline stays the full store: filtering is a wire concern, applied on
 * the way out, so main's idea of what a renderer holds never depends on which
 * renderer it is. A target simply has no key for a slice it is not routed —
 * absence rather than a neutral value, so a slice that starts being read shows
 * up as missing instead of silently reading as empty forever. The drift
 * watchdog compares absent against absent and stays quiet.
 */

import type { RuntimeSliceKey } from './runtime-store'
import type { RuntimePatchBatch } from './runtime-patch'
import type { LayoutUpdateData } from './types'

export type SceneTarget = 'canvas-bg' | 'above-view' | 'agent-layer'

export const SCENE_TARGETS = ['canvas-bg', 'above-view', 'agent-layer'] as const

/**
 * The `LayoutUpdateData` fields each slice owns, so filtering the snapshot and
 * filtering the patch stream stay two views of one decision.
 */
const SLICE_LAYOUT_KEYS: Record<RuntimeSliceKey, readonly (keyof LayoutUpdateData)[]> = {
  camera: ['zoom', 'pan', 'cameraTransitionStartedAt'],
  chrome: [
    'windowWidth',
    'canvasOrigin',
    'leftChromeWidth',
    'toolbarCenterX',
    'devtoolsOpen',
    'devtoolsWidth',
  ],
  scene: ['entityOrder', 'entities', 'groups'],
  selection: [
    'selectedEntityIds',
    'selectionOperandIds',
    'selection',
    'selectedGroupId',
  ],
  tool: ['activeTool', 'toolDefaults', 'pendingPlacement'],
  focus: ['keyboardTargetPageId', 'interactivePageId', 'focusPresentation'],
  hover: ['hover'],
  interaction: ['interaction'],
  inspect: ['inspect'],
  annotations: ['annotations'],
  edges: ['edges'],
  fixProgress: ['fixProgress'],
  presence: ['presenceCursors'],
  pageScroll: ['pageScroll'],
  annotationBboxes: ['annotationBboxes'],
  idle: ['idle'],
}

/**
 * What each renderer actually reads, checked against its source.
 *
 * `camera`, `chrome`, and `scene` are on every list because the snapshot
 * projection is built from them — a target without them has no coordinate
 * system and no entities, which is not a renderer.
 */
const SCENE_TARGET_SLICES: Record<SceneTarget, readonly RuntimeSliceKey[]> = {
  // Page chrome, device shells, group backgrounds, the grid, the placement
  // preview. No hover outline, no gestures, no annotation geometry.
  // `idle` so inline HTML files can stop animating when nobody is looking.
  'canvas-bg': ['camera', 'chrome', 'scene', 'selection', 'tool', 'focus', 'annotations', 'idle'],
  // The interaction surface: everything except the inspect tree and presence
  // cursors, which render in the agent overlay window.
  'above-view': [
    'camera',
    'chrome',
    'scene',
    'selection',
    'tool',
    'focus',
    'hover',
    'interaction',
    'annotations',
    'edges',
    'fixProgress',
    'pageScroll',
    'annotationBboxes',
  ],
  // Agent presence cursors and the inspect popover, clipped to the canvas.
  'agent-layer': ['camera', 'chrome', 'scene', 'inspect', 'presence'],
}

export function omittedSlicesFor(target: SceneTarget): RuntimeSliceKey[] {
  const routed = new Set(SCENE_TARGET_SLICES[target])
  return (Object.keys(SLICE_LAYOUT_KEYS) as RuntimeSliceKey[]).filter(
    (slice) => !routed.has(slice),
  )
}

/** The snapshot keys `target` is not routed — what filtering removes. */
function omittedLayoutKeysFor(target: SceneTarget): (keyof LayoutUpdateData)[] {
  return omittedSlicesFor(target).flatMap((slice) => [...SLICE_LAYOUT_KEYS[slice]])
}

/**
 * The snapshot as `target` sees it. Typed as a whole `LayoutUpdateData` because
 * that is what the channel carries; the fields dropped are exactly the ones
 * this target never reads.
 */
export function filterSceneSnapshot(
  data: LayoutUpdateData,
  target: SceneTarget,
): LayoutUpdateData {
  const omitted = omittedLayoutKeysFor(target)
  if (omitted.length === 0) return data
  const filtered = { ...data } as Partial<LayoutUpdateData>
  for (const key of omitted) delete filtered[key]
  return filtered as LayoutUpdateData
}

/** The batch as `target` sees it, or null when nothing in it was for this
 *  target. Entity patches always cross — every target draws entities. */
export function filterPatchBatch(
  batch: RuntimePatchBatch,
  target: SceneTarget,
): RuntimePatchBatch | null {
  const routed = new Set(SCENE_TARGET_SLICES[target])
  const patches = batch.patches.filter(
    (patch) => patch.kind !== 'slice' || routed.has(patch.slice),
  )
  if (patches.length === 0) return null
  if (patches.length === batch.patches.length) return batch
  return { ...batch, patches }
}
