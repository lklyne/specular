/**
 * The diffed runtime store: its two projections, the diff that produces
 * patches, and the reducer that applies them.
 *
 * The load-bearing property is convergence. Patches are allowed to be lossy
 * only because the next full `layoutUpdate` heals whatever they missed, so a
 * store that has taken a patch stream — including a dropped patch — must end up
 * byte-identical to a store built from the snapshot alone. Everything else here
 * exists to make that property meaningful: a diff that under-reports would let
 * a renderer stay wrong, and a diff that over-reports would put the scene back
 * on the wire it was taken off.
 *
 * Mutation-verified by:
 * - returning `[]` from the entity loop in `diffRuntimeStores` — the add,
 *   remove, and move tests fail;
 * - dropping the `deepEqual` guard on slices so every slice is emitted — the
 *   "quiet when nothing moved" and minimal-patch tests fail;
 * - having `applyRuntimePatch` mutate `store.entities` in place instead of
 *   copying — the "leaves the previous store untouched" test fails;
 * - dropping `entityIds` from `SceneSlice` — the round-trip test fails;
 * - making the renderer store's `applySnapshot` merge instead of replace — the
 *   convergence test fails.
 */

import { describe, expect, it } from 'vitest'
import {
  RUNTIME_SLICE_KEYS,
  snapshotToStore,
  storeToLayoutData,
  type RuntimeStore,
} from '../../src/shared/runtime-store'
import { applyRuntimePatch, type RuntimePatch } from '../../src/shared/runtime-patch'
import { diffRuntimeStores } from '../../src/shared/runtime-store-diff'
import { createRuntimeStore } from '../../src/renderer/shared/runtime-store'
import { DEFAULT_TOOL_DEFAULTS } from '../../src/shared/tool-defaults'
import type { CanvasSceneTextEntity, LayoutUpdateData } from '../../src/shared/types'

function textEntity(id: string, x: number): CanvasSceneTextEntity {
  return {
    kind: 'text',
    id,
    text: id,
    color: '#111111',
    textStyle: 'plain',
    widthMode: 'auto',
    canvasX: x,
    canvasY: 0,
    width: 200,
    height: 60,
    screenX: x,
    screenY: 0,
    screenWidth: 200,
    screenHeight: 60,
  }
}

function snapshot(overrides: Partial<LayoutUpdateData> = {}): LayoutUpdateData {
  const entities = overrides.entities ?? [textEntity('text-a', 0), textEntity('text-b', 400)]
  return {
    windowWidth: 1440,
    zoom: 1,
    pan: { x: 0, y: 0 },
    canvasOrigin: { x: 0, y: 40 },
    leftChromeWidth: 0,
    toolbarCenterX: 720,
    entityOrder: entities.map((entity) => entity.id),
    entities,
    selectedEntityIds: [],
    selectionOperandIds: [],
    selection: [],
    activeTool: { kind: 'select' },
    toolDefaults: DEFAULT_TOOL_DEFAULTS,
    annotations: [],
    inspect: null,
    fixProgress: {},
    selectedGroupId: null,
    hover: null,
    interaction: { kind: 'idle' },
    pendingPlacement: null,
    devtoolsOpen: false,
    devtoolsWidth: 0,
    edges: [],
    groups: [],
    presenceCursors: [],
    keyboardTargetPageId: null,
    interactivePageId: null,
    focusPresentation: null,
    cameraTransitionStartedAt: null,
    pageScroll: {},
    annotationBboxes: {},
    ...overrides,
  }
}

describe('runtime store projections', () => {
  it('round-trips a layout snapshot through the normalized form', () => {
    const data = snapshot()
    expect(storeToLayoutData(snapshotToStore(data))).toEqual(data)
  })

  it('keys every scene entity by id and preserves emission order', () => {
    const store = snapshotToStore(snapshot())
    expect(Object.keys(store.entities).sort()).toEqual(['text-a', 'text-b'])
    expect(store.slices.scene.entityIds).toEqual(['text-a', 'text-b'])
  })
})

describe('diffRuntimeStores', () => {
  it('says nothing when nothing moved', () => {
    const before = snapshotToStore(snapshot())
    const after = snapshotToStore(snapshot())
    expect(diffRuntimeStores(before, after)).toEqual([])
  })

  it('emits one entity patch when one entity moved', () => {
    const before = snapshotToStore(snapshot())
    const after = snapshotToStore(
      snapshot({ entities: [textEntity('text-a', 0), textEntity('text-b', 999)] }),
    )
    expect(diffRuntimeStores(before, after)).toEqual([
      { kind: 'entity', id: 'text-b', entity: textEntity('text-b', 999) },
    ])
  })

  it('emits the added entity plus the order that admitted it', () => {
    const before = snapshotToStore(snapshot())
    const added = textEntity('text-c', 800)
    const after = snapshotToStore(
      snapshot({ entities: [textEntity('text-a', 0), textEntity('text-b', 400), added] }),
    )
    const patches = diffRuntimeStores(before, after)

    expect(patches).toContainEqual({ kind: 'entity', id: 'text-c', entity: added })
    expect(patches.filter((patch) => patch.kind === 'entity')).toHaveLength(1)
    expect(patches.filter((patch) => patch.kind === 'slice').map((patch) => patch.slice)).toEqual([
      'scene',
    ])
  })

  it('emits a null entity for a removal', () => {
    const before = snapshotToStore(snapshot())
    const after = snapshotToStore(snapshot({ entities: [textEntity('text-a', 0)] }))
    expect(diffRuntimeStores(before, after)).toContainEqual({
      kind: 'entity',
      id: 'text-b',
      entity: null,
    })
  })

  it('emits only the slice that changed', () => {
    const before = snapshotToStore(snapshot())
    const after = snapshotToStore(snapshot({ hover: { id: 'text-a', kind: 'text' } }))
    expect(diffRuntimeStores(before, after)).toEqual([
      { kind: 'slice', slice: 'hover', value: { id: 'text-a', kind: 'text' } },
    ])
  })

  it('ignores buildMs, which times the pass rather than the scene', () => {
    const before = snapshotToStore(snapshot({ buildMs: 1 }))
    const after = snapshotToStore(snapshot({ buildMs: 42 }))
    expect(diffRuntimeStores(before, after)).toEqual([])
  })
})

describe('applyRuntimePatch', () => {
  it('replaces the named slice and leaves the previous store untouched', () => {
    const before = snapshotToStore(snapshot())
    const after = applyRuntimePatch(before, {
      kind: 'slice',
      slice: 'hover',
      value: { id: 'text-a', kind: 'text' },
    })

    expect(after.slices.hover).toEqual({ id: 'text-a', kind: 'text' })
    expect(before.slices.hover).toBeNull()
    expect(after.slices.camera).toBe(before.slices.camera)
  })

  it('adds, replaces, and removes one entity at a time', () => {
    const before = snapshotToStore(snapshot())
    const added = textEntity('text-c', 800)

    const withAdded = applyRuntimePatch(before, { kind: 'entity', id: 'text-c', entity: added })
    expect(withAdded.entities['text-c']).toBe(added)
    expect(before.entities['text-c']).toBeUndefined()

    const moved = textEntity('text-a', 12)
    const withMoved = applyRuntimePatch(withAdded, { kind: 'entity', id: 'text-a', entity: moved })
    expect(withMoved.entities['text-a']).toBe(moved)
    expect(withMoved.entities['text-b']).toBe(before.entities['text-b'])

    const withRemoved = applyRuntimePatch(withMoved, {
      kind: 'entity',
      id: 'text-b',
      entity: null,
    })
    expect(withRemoved.entities['text-b']).toBeUndefined()
    expect(Object.keys(withRemoved.entities).sort()).toEqual(['text-a', 'text-c'])
  })

  it('returns the same store when the patch repeats what is held', () => {
    const before = snapshotToStore(snapshot())
    const held = before.slices.selection
    expect(applyRuntimePatch(before, { kind: 'slice', slice: 'selection', value: held })).toBe(
      before,
    )
    expect(applyRuntimePatch(before, { kind: 'entity', id: 'nope', entity: null })).toBe(before)
  })

  it('round-trips a whole diff back onto the store it was taken from', () => {
    const before = snapshotToStore(snapshot())
    const target = snapshotToStore(
      snapshot({
        entities: [textEntity('text-a', 33), textEntity('text-c', 800)],
        hover: { id: 'text-c', kind: 'text' },
        zoom: 2,
      }),
    )

    let rebuilt: RuntimeStore = before
    for (const patch of diffRuntimeStores(before, target)) {
      rebuilt = applyRuntimePatch(rebuilt, patch)
    }

    expect(rebuilt).toEqual(target)
  })
})

describe('patch-then-snapshot convergence', () => {
  it('heals a store that dropped a patch when the next snapshot lands', () => {
    const initial = snapshot()
    const store = createRuntimeStore(initial)

    const next = snapshot({
      entities: [textEntity('text-a', 55), textEntity('text-b', 400), textEntity('text-c', 800)],
      hover: { id: 'text-c', kind: 'text' },
      selectedEntityIds: ['text-c'],
      zoom: 1.5,
    })
    const patches = diffRuntimeStores(snapshotToStore(initial), snapshotToStore(next))
    expect(patches.length).toBeGreaterThan(1)

    // Drop one, the way a renderer that missed a send would.
    const dropped = patches[0]
    store.applyPatches({ patches: patches.filter((patch) => patch !== dropped) })
    expect(store.read()).not.toEqual(snapshotToStore(next))

    store.applySnapshot(next)

    expect(store.read()).toEqual(snapshotToStore(next))
    expect(store.readLayoutData()).toEqual(next)
  })

  it('keeps slice identity across a snapshot that repeats what is held', () => {
    const initial = snapshot()
    const store = createRuntimeStore(initial)
    const held = store.read()

    store.applySnapshot(snapshot())

    for (const slice of RUNTIME_SLICE_KEYS) {
      expect(store.read().slices[slice]).toBe(held.slices[slice])
    }
  })

  it('leaves the layout projection identical when only hover moved', () => {
    const store = createRuntimeStore(snapshot())
    const projected = store.readLayoutData()

    const patch: RuntimePatch = {
      kind: 'slice',
      slice: 'hover',
      value: { id: 'text-a', kind: 'text' },
    }
    store.applyPatches({ patches: [patch] })

    expect(store.read().slices.hover).toEqual({ id: 'text-a', kind: 'text' })
    // Hover has its own subscription, so it is excluded from the projection's
    // identity — otherwise a pointer move re-renders every layoutData consumer.
    expect(store.readLayoutData()).toBe(projected)
  })
})
