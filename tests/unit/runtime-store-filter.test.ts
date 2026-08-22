/**
 * Per-renderer slice routing (diffed-runtime-store, phase 3).
 *
 * One payload used to reach every canvas renderer, so `inspect` — the selected
 * page's whole component tree, and most of the payload's bytes — was paid for
 * three times over for the one renderer that draws from it. Routing is what
 * turns "each renderer reads a handful of slices" into what each renderer
 * costs.
 *
 * Two properties hold the rest of the system up. A routed slice is *absent*
 * rather than emptied, so the drift watchdog compares absent against absent and
 * a slice that starts being read fails loudly instead of reading as empty
 * forever. And `camera`/`chrome`/`scene` reach every target, because the flat
 * projection is built from them.
 *
 * Mutation-verified by:
 * - adding `inspect` to above-view's list in `SCENE_TARGET_SLICES` — the
 *   omission and snapshot tests fail;
 * - dropping the `patch.kind !== 'slice'` term from `filterPatchBatch` — the
 *   entity-patch test fails (entity patches stop crossing);
 * - returning `data` unchanged from `filterSceneSnapshot` — the snapshot tests
 *   fail;
 * - removing `scene` from agent-layer's list — the every-target test fails.
 */

import { describe, expect, it } from 'vitest'
import {
  SCENE_TARGETS,
  filterPatchBatch,
  filterSceneSnapshot,
  omittedSlicesFor,
} from '../../src/shared/runtime-store-filter'
import type { RuntimePatchBatch } from '../../src/shared/runtime-patch'
import { DEFAULT_TOOL_DEFAULTS } from '../../src/shared/tool-defaults'
import type { CanvasSceneTextEntity, LayoutUpdateData } from '../../src/shared/types'

function textEntity(id: string): CanvasSceneTextEntity {
  return {
    kind: 'text',
    id,
    text: id,
    color: '#111111',
    textStyle: 'plain',
    widthMode: 'auto',
    canvasX: 0,
    canvasY: 0,
    width: 200,
    height: 60,
    screenX: 0,
    screenY: 0,
    screenWidth: 200,
    screenHeight: 60,
  }
}

function snapshot(): LayoutUpdateData {
  return {
    windowWidth: 1440,
    zoom: 1,
    pan: { x: 0, y: 0 },
    canvasOrigin: { x: 0, y: 40 },
    leftChromeWidth: 0,
    toolbarCenterX: 720,
    entityOrder: ['text-a'],
    entities: [textEntity('text-a')],
    selectedEntityIds: [],
    selectionOperandIds: [],
    selection: [],
    activeSelection: null,
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
  }
}

describe('omittedSlicesFor', () => {
  it('keeps the inspect tree off every renderer but the agent layer', () => {
    expect(omittedSlicesFor('canvas-bg')).toContain('inspect')
    expect(omittedSlicesFor('above-view')).toContain('inspect')
    expect(omittedSlicesFor('agent-layer')).not.toContain('inspect')
  })

  it('keeps presence cursors off the renderers that do not draw them', () => {
    expect(omittedSlicesFor('canvas-bg')).toContain('presence')
    expect(omittedSlicesFor('above-view')).toContain('presence')
    expect(omittedSlicesFor('agent-layer')).not.toContain('presence')
  })

  it('routes the slices the flat projection is built from to every target', () => {
    for (const target of SCENE_TARGETS) {
      expect(omittedSlicesFor(target)).not.toContain('camera')
      expect(omittedSlicesFor(target)).not.toContain('chrome')
      expect(omittedSlicesFor(target)).not.toContain('scene')
    }
  })
})

describe('filterSceneSnapshot', () => {
  it('drops the keys of unrouted slices and leaves the source untouched', () => {
    const data = snapshot()
    const filtered = filterSceneSnapshot(data, 'canvas-bg')

    expect(filtered).not.toHaveProperty('inspect')
    expect(filtered).not.toHaveProperty('presenceCursors')
    expect(filtered).not.toHaveProperty('hover')
    expect(filtered).toHaveProperty('annotations')
    expect(filtered.entities).toBe(data.entities)
    expect(data).toHaveProperty('inspect')
  })

  it('gives the agent layer the inspect tree and nothing it does not read', () => {
    const filtered = filterSceneSnapshot(snapshot(), 'agent-layer')

    expect(filtered).toHaveProperty('inspect')
    expect(filtered).toHaveProperty('presenceCursors')
    expect(filtered).not.toHaveProperty('annotations')
    expect(filtered).not.toHaveProperty('edges')
    expect(filtered).not.toHaveProperty('selectedEntityIds')
  })
})

describe('filterPatchBatch', () => {
  const batch: RuntimePatchBatch = {
    patches: [
      { kind: 'slice', slice: 'hover', value: { id: 'text-a', kind: 'text' } },
      { kind: 'entity', id: 'text-a', entity: textEntity('text-a') },
    ],
    buildMs: 4,
  }

  it('sends entity patches to every target and slice patches only to readers', () => {
    expect(filterPatchBatch(batch, 'above-view')).toBe(batch)
    expect(filterPatchBatch(batch, 'canvas-bg')?.patches).toEqual([batch.patches[1]])
  })

  it('says nothing to a target that reads none of the batch', () => {
    const hoverOnly: RuntimePatchBatch = { patches: [batch.patches[0]] }
    expect(filterPatchBatch(hoverOnly, 'canvas-bg')).toBeNull()
  })
})
