/**
 * The normalized form of `LayoutUpdateData`.
 *
 * A layout snapshot is one flat record whose every field is rebuilt on every
 * pass, so the smallest change — a hover, one entity moving a pixel — hands
 * every consumer a whole new scene. Normalizing splits it into two axes that
 * can change independently: a map of scene entities keyed by id, and a handful
 * of small slices for everything that is not an entity. A change then names
 * exactly what it touched, which is what makes a diff worth sending
 * (`runtime-store-diff.ts`) and a subscription worth scoping (`useSlice`).
 *
 * The two projections are lossless in both directions: `snapshotToStore` and
 * `storeToLayoutData` round-trip, so `layoutUpdate` stays the reconcile
 * baseline that patched state heals against.
 */

import type { CanvasSceneEntity, CanvasSceneGroupEntity, LayoutUpdateData } from './types'

/** Camera transform. Pan and zoom also ride `viewportNudge`, which stays its
 *  own channel — a camera move over an unchanged scene is not a scene edit. */
type CameraSlice = Pick<
  LayoutUpdateData,
  'zoom' | 'pan' | 'cameraTransitionStartedAt'
>

/** Window furniture the canvas coordinate system is measured against. */
type ChromeSlice = Pick<
  LayoutUpdateData,
  | 'windowWidth'
  | 'canvasOrigin'
  | 'leftChromeWidth'
  | 'toolbarCenterX'
  | 'devtoolsOpen'
  | 'devtoolsWidth'
>

/**
 * The orderings the entity map cannot express on its own: `entityOrder` is the
 * back-to-front stack across nodes *and* edges, `entityIds` is the order the
 * scene array is emitted in, and `groupIds` is the separate order the `groups`
 * projection uses.
 */
interface SceneSlice {
  entityOrder: string[]
  entityIds: string[]
  groupIds: string[]
}

type SelectionSlice = Pick<
  LayoutUpdateData,
  | 'selectedEntityIds'
  | 'selectionOperandIds'
  | 'selection'
  | 'activeSelection'
  | 'selectedGroupId'
>

type ToolSlice = Pick<
  LayoutUpdateData,
  'activeTool' | 'toolDefaults' | 'pendingPlacement'
>

/** Where keyboard and forwarded input go, and the focus-presentation override. */
type FocusSlice = Pick<
  LayoutUpdateData,
  'keyboardTargetPageId' | 'interactivePageId' | 'focusPresentation'
>

export interface RuntimeStoreSlices {
  camera: CameraSlice
  chrome: ChromeSlice
  scene: SceneSlice
  selection: SelectionSlice
  tool: ToolSlice
  focus: FocusSlice
  hover: LayoutUpdateData['hover']
  interaction: LayoutUpdateData['interaction']
  inspect: LayoutUpdateData['inspect']
  annotations: LayoutUpdateData['annotations']
  edges: LayoutUpdateData['edges']
  fixProgress: LayoutUpdateData['fixProgress']
  presence: LayoutUpdateData['presenceCursors']
  pageScroll: LayoutUpdateData['pageScroll']
  annotationBboxes: LayoutUpdateData['annotationBboxes']
  idle: LayoutUpdateData['idle']
}

export type RuntimeSliceKey = keyof RuntimeStoreSlices

export const RUNTIME_SLICE_KEYS = [
  'camera',
  'chrome',
  'scene',
  'selection',
  'tool',
  'focus',
  'hover',
  'interaction',
  'inspect',
  'annotations',
  'edges',
  'fixProgress',
  'presence',
  'pageScroll',
  'annotationBboxes',
  'idle',
] as const satisfies readonly RuntimeSliceKey[]

export interface RuntimeStore {
  /** Every scene entity by id, including the group entities `groups` projects. */
  entities: Record<string, CanvasSceneEntity>
  /**
   * Partial because a renderer holds only the slices it is routed
   * (`runtime-store-filter.ts`). Main's baseline always holds all of them.
   */
  slices: Partial<RuntimeStoreSlices>
  /**
   * Cost of the layout pass that produced this store. Diagnostic only (the
   * canvas perf HUD reads it), and different on every build by construction,
   * so it is deliberately outside the slices: diffing it would make every pass
   * emit a patch for a number no consumer treats as scene content.
   */
  buildMs?: number
}

export function snapshotToStore(data: LayoutUpdateData): RuntimeStore {
  const entities: Record<string, CanvasSceneEntity> = {}
  for (const entity of data.entities) entities[entity.id] = entity
  const groups = data.groups ?? []
  for (const group of groups) entities[group.id] ??= group
  return {
    entities,
    ...(data.buildMs != null ? { buildMs: data.buildMs } : {}),
    slices: {
      camera: {
        zoom: data.zoom,
        pan: data.pan,
        cameraTransitionStartedAt: data.cameraTransitionStartedAt,
      },
      chrome: {
        windowWidth: data.windowWidth,
        canvasOrigin: data.canvasOrigin,
        leftChromeWidth: data.leftChromeWidth,
        toolbarCenterX: data.toolbarCenterX,
        devtoolsOpen: data.devtoolsOpen,
        devtoolsWidth: data.devtoolsWidth,
      },
      scene: {
        entityOrder: data.entityOrder,
        entityIds: data.entities.map((entity) => entity.id),
        groupIds: groups.map((group) => group.id),
      },
      selection: {
        selectedEntityIds: data.selectedEntityIds,
        selectionOperandIds: data.selectionOperandIds,
        selection: data.selection,
        activeSelection: data.activeSelection,
        selectedGroupId: data.selectedGroupId ?? null,
      },
      tool: {
        activeTool: data.activeTool,
        toolDefaults: data.toolDefaults,
        pendingPlacement: data.pendingPlacement,
      },
      focus: {
        keyboardTargetPageId: data.keyboardTargetPageId,
        interactivePageId: data.interactivePageId,
        focusPresentation: data.focusPresentation,
      },
      hover: data.hover,
      interaction: data.interaction,
      inspect: data.inspect,
      annotations: data.annotations,
      edges: data.edges,
      fixProgress: data.fixProgress,
      presence: data.presenceCursors,
      pageScroll: data.pageScroll,
      annotationBboxes: data.annotationBboxes,
      idle: data.idle,
    },
  }
}

/**
 * The flat projection, for the consumers that still read the whole payload.
 *
 * Fields belonging to a slice this store was never routed project as absent —
 * which is sound because they are exactly the fields that store's renderer
 * never reads. The result is asserted whole rather than each field being
 * widened, so a consumer reading the projection sees the shape main sends.
 */
export function storeToLayoutData(store: RuntimeStore): LayoutUpdateData {
  const { camera, chrome, scene, selection, tool, focus } = store.slices
  const entities: CanvasSceneEntity[] = []
  for (const id of scene?.entityIds ?? []) {
    const entity = store.entities[id]
    if (entity) entities.push(entity)
  }
  const groups: CanvasSceneGroupEntity[] = []
  for (const id of scene?.groupIds ?? []) {
    const entity = store.entities[id]
    if (entity?.kind === 'group') groups.push(entity)
  }
  return {
    ...(store.buildMs != null ? { buildMs: store.buildMs } : {}),
    windowWidth: chrome?.windowWidth,
    zoom: camera?.zoom,
    pan: camera?.pan,
    canvasOrigin: chrome?.canvasOrigin,
    leftChromeWidth: chrome?.leftChromeWidth,
    toolbarCenterX: chrome?.toolbarCenterX,
    entityOrder: scene?.entityOrder,
    entities,
    selectedEntityIds: selection?.selectedEntityIds,
    selectionOperandIds: selection?.selectionOperandIds,
    selection: selection?.selection,
    activeSelection: selection?.activeSelection,
    activeTool: tool?.activeTool,
    toolDefaults: tool?.toolDefaults,
    annotations: store.slices.annotations,
    inspect: store.slices.inspect,
    fixProgress: store.slices.fixProgress,
    selectedGroupId: selection?.selectedGroupId,
    hover: store.slices.hover,
    interaction: store.slices.interaction,
    pendingPlacement: tool?.pendingPlacement,
    devtoolsOpen: chrome?.devtoolsOpen,
    devtoolsWidth: chrome?.devtoolsWidth,
    edges: store.slices.edges,
    groups,
    presenceCursors: store.slices.presence,
    idle: store.slices.idle,
    keyboardTargetPageId: focus?.keyboardTargetPageId,
    interactivePageId: focus?.interactivePageId,
    focusPresentation: focus?.focusPresentation,
    cameraTransitionStartedAt: camera?.cameraTransitionStartedAt,
    pageScroll: store.slices.pageScroll,
    annotationBboxes: store.slices.annotationBboxes,
  } as LayoutUpdateData
}
