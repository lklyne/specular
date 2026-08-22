/**
 * Hover leaves the layout pass (diffed-runtime-store, phase 1).
 *
 * `setHoveredPage` / `setHoverEntity` used to `markDirty('canvas')` and
 * `requestLayout()`, so every pointer move over the canvas paid for a full
 * scene rebuild, serialize, and broadcast. They now push one `runtime-patch`
 * instead. The snapshot still has to carry the truth — that is the reconcile
 * baseline the renderer store heals against — so `buildCanvasLayoutData` must
 * keep reporting the current hover for a pass triggered by anything else.
 *
 * Mutation-verified by:
 * - restoring `markDirty('canvas') + requestLayout()` in `commitHoverTarget`
 *   (runtime-core.ts) — the two "no layout pass" assertions fail;
 * - dropping the `broadcastRuntimePatch` call there — the patch assertions fail;
 * - dropping its `sameHoverTarget` early return — the repeated-hover test fails;
 * - dropping `hover: hoverTarget` from the `buildCanvasLayoutData` literal
 *   (canvas-layout-data.ts) — the snapshot assertion fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import { ipcChannels } from '../../src/shared/ipc-contract'
import type { RuntimePatchBatch } from '../../src/shared/runtime-patch'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import { setHoverEntity, setHoveredPage } from '../../src/main/runtime/runtime-core'
import { createTextEntity } from '../../src/main/runtime/text-entity-state'
import { clearAllDirty, isDirty } from '../../src/main/runtime/layout-dirty'
import { layoutCache } from '../../src/main/runtime/layout-cache'

let harness: WorkspaceHarness

const PAGE_ID = 'hover-host-page'

function hostPageNode(): JsonCanvasLinkNode {
  return {
    id: PAGE_ID,
    type: 'link',
    x: 100,
    y: 100,
    width: 375,
    height: 667,
    url: 'https://example.com/hover',
    presetIndex: 0,
  }
}

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Hover host',
    doc: {
      nodes: [hostPageNode()],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

/** Arm the observables a layout pass would move, so a hover can be watched.
 *  Fixture loading and entity creation legitimately dirty the canvas and arm
 *  the debounce; the harness keeps the pass dormant, so they have to be reset
 *  by hand before the hover under test runs. */
function armLayoutWatch(): void {
  harness.clearBroadcasts()
  clearAllDirty()
  if (layoutCache.layoutTimer) {
    clearTimeout(layoutCache.layoutTimer)
    layoutCache.layoutTimer = null
  }
}

/** `requestLayout()` debounces onto this handle, so an armed timer is a pass. */
function layoutPassRequested(): boolean {
  return layoutCache.layoutTimer !== null || isDirty('canvas')
}

/** The patch stream as one renderer sees it. Every canvas renderer gets the
 *  same batch, so read a single target rather than counting sends. */
function patches() {
  const sends = harness.broadcasts.filter((b) => b.channel === ipcChannels.runtimePatch)
  const target = sends[0]?.webContentsId
  return sends
    .filter((send) => send.webContentsId === target)
    .flatMap((send) => (send.args[0] as RuntimePatchBatch).patches)
}

function patchTargets() {
  return new Set(
    harness.broadcasts
      .filter((b) => b.channel === ipcChannels.runtimePatch)
      .map((b) => b.webContentsId),
  )
}

describe('hover rides a runtime patch, not a layout pass', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('emits one patch per page-hover change and requests no layout pass', () => {
    loadHostPage()
    armLayoutWatch()

    setHoveredPage(PAGE_ID)

    expect(patches()).toEqual([
      { kind: 'slice', slice: 'hover', value: { id: PAGE_ID, kind: 'page' } },
    ])
    expect(patchTargets().size).toBe(2)
    expect(harness.broadcasts.every((b) => b.channel === ipcChannels.runtimePatch)).toBe(true)
    expect(layoutPassRequested()).toBe(false)
  })

  it('says nothing when the hover target has not moved', () => {
    loadHostPage()
    setHoveredPage(PAGE_ID)
    armLayoutWatch()

    setHoveredPage(PAGE_ID)

    expect(patches()).toHaveLength(0)
    expect(layoutPassRequested()).toBe(false)
  })

  it('patches non-page entities and the cleared hover the same way', () => {
    loadHostPage()
    const text = createTextEntity({ canvasX: 0, canvasY: 0, text: 'hoverable' })
    armLayoutWatch()

    setHoverEntity({ id: text.id, kind: 'text' })
    setHoveredPage(null)

    expect(patches()).toEqual([
      { kind: 'slice', slice: 'hover', value: { id: text.id, kind: 'text' } },
      { kind: 'slice', slice: 'hover', value: null },
    ])
    expect(layoutPassRequested()).toBe(false)
  })

  it('still carries the current hover in a full layout snapshot', () => {
    loadHostPage()

    setHoveredPage(PAGE_ID)
    expect(getCanvasLayoutData().hover).toEqual({ id: PAGE_ID, kind: 'page' })

    setHoveredPage(null)
    expect(getCanvasLayoutData().hover).toBeNull()
  })
})
