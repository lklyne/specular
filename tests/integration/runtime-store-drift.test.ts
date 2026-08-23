/**
 * The patch stream alone has to be enough.
 *
 * Snapshots exist so a dropped patch heals (ADR 0036 §3), which only works as a
 * safety net if it is never also the delivery mechanism: the moment a snapshot
 * is the only carrier for a change, main has no way to tell a renderer that
 * converged from one that has been stale for a second, and the drift watchdog
 * counts honest updates as corruption. So a renderer that applies every send in
 * order must already hold main's truth *before* each snapshot lands.
 *
 * Mutation-verified by:
 * - sending the snapshot instead of the pass's patches when the interval is due
 *   (the previous `broadcastSceneUpdate`) — the drift assertions fail with the
 *   selection, interaction and entity cells the snapshot smuggled in;
 * - having a forced snapshot skip the patch batch (`fanOut`) — the re-seat at
 *   the end of the first test fails with the entity it smuggled in;
 * - having `seatSceneBootstrap` filter the payload without running it through
 *   the pass fan-out — the bootstrap test fails, because main keeps diffing
 *   against a store no renderer holds.
 */

import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { ipcChannels } from '../../src/shared/ipc-contract'
import type { RuntimePatchBatch } from '../../src/shared/runtime-patch'
import type { LayoutUpdateData } from '../../src/shared/types'
import { snapshotToStore } from '../../src/shared/runtime-store'
import { diffRuntimeStores } from '../../src/shared/runtime-store-diff'
import type { SceneTarget } from '../../src/shared/runtime-store-filter'
import { createRuntimeStore } from '../../src/renderer/shared/runtime-store'
import { WebContentsView } from './electron-stub'
import { aboveView, bgView, setCursorOverlayWindow } from '../../src/main/runtime/view-refs'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import {
  broadcastSceneSnapshot,
  broadcastSceneUpdate,
  seatSceneBootstrap,
} from '../../src/main/runtime/runtime-patch-broadcast'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { createTextEntity } from '../../src/main/runtime/text-entity-state'
import { selectEntities, selectNone } from '../../src/main/runtime/selection-controller'
import {
  beginDraggingEntities,
  clearInteractionState,
} from '../../src/main/runtime/interaction-state'

let harness: WorkspaceHarness

const realNow = Date.now.bind(Date)
let clockSkew = 0

/**
 * One renderer's copy of the store, fed exactly what main sent it, plus the
 * dev watchdog's question: did the snapshot carry anything the patch stream
 * had not already delivered?
 */
class Renderer {
  readonly store = createRuntimeStore()
  readonly drift: string[] = []
  private seeded = false

  constructor(
    readonly target: SceneTarget,
    readonly webContentsId: number,
  ) {}

  receive(channel: string, payload: unknown): void {
    if (channel === ipcChannels.layoutUpdate) {
      const incoming = snapshotToStore(payload as LayoutUpdateData)
      if (this.seeded) {
        for (const cell of diffRuntimeStores(this.store.read(), incoming)) {
          this.drift.push(
            cell.kind === 'slice'
              ? `${this.target}:slice:${cell.slice}`
              : `${this.target}:entity:${cell.id}`,
          )
        }
      }
      this.store.applySnapshot(payload as LayoutUpdateData)
      this.seeded = true
      return
    }
    if (channel === ipcChannels.runtimePatch) {
      this.store.applyPatches(payload as RuntimePatchBatch)
    }
  }
}

let renderers: Renderer[] = []

/** Hand every renderer what main sent it, in send order, then start fresh. */
function deliver(): void {
  for (const record of harness.broadcasts) {
    const renderer = renderers.find((r) => r.webContentsId === record.webContentsId)
    renderer?.receive(record.channel, record.args[0])
  }
  harness.clearBroadcasts()
}

/** One layout pass's fan-out, delivered. */
function pass(): void {
  broadcastSceneUpdate(getCanvasLayoutData())
  deliver()
}

function drift(): string[] {
  return renderers.flatMap((renderer) => renderer.drift)
}

function createPage(url: string): string {
  return applyCanvasPatch({
    entities: [{ kind: 'page', url, canvasX: 0, canvasY: 0, presetIndex: 2 }],
  }).created[0]
}

describe('runtime store drift', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
    clearInteractionState()

    // The agent overlay is a child window rather than a view, so the harness's
    // view topology doesn't include it — and `inspect` only ever reaches it.
    const overlay = new WebContentsView()
    setCursorOverlayWindow({
      webContents: overlay.webContents,
      isDestroyed: () => false,
    } as never)

    clockSkew = 0
    vi.spyOn(Date, 'now').mockImplementation(() => realNow() + clockSkew)

    renderers = [
      new Renderer('canvas-bg', bgView!.webContents.id),
      new Renderer('above-view', aboveView!.webContents.id),
      new Renderer('agent-layer', overlay.webContents.id),
    ]
    harness.clearBroadcasts()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    setCursorOverlayWindow(null)
  })

  afterAll(() => harness?.dispose())

  it('leaves no renderer behind across a create burst, a selection, and a drag', () => {
    broadcastSceneSnapshot(getCanvasLayoutData())
    deliver()

    const pageIds = ['/one', '/two', '/three'].map((path) =>
      createPage(`https://example.com${path}`),
    )
    pass()

    // A pass that lands after the snapshot interval is the one that used to
    // swallow its own delta.
    clockSkew += 2000
    selectEntities(pageIds.slice(0, 2))
    pass()

    beginDraggingEntities(pageIds.slice(0, 2))
    pass()

    clockSkew += 2000
    clearInteractionState()
    const text = createTextEntity({ canvasX: 40, canvasY: 60, text: 'after the drag' })
    selectEntities([text.id])
    pass()

    // Window init re-seats every renderer mid-session, with truth moved on
    // since the last pass. The re-seat still carries nothing new.
    createTextEntity({ canvasX: 200, canvasY: 0, text: 'before the re-seat' })
    broadcastSceneSnapshot(getCanvasLayoutData())
    deliver()

    expect(drift()).toEqual([])
  })

  it('answers a bootstrap with a seed the bus has already delivered', () => {
    broadcastSceneSnapshot(getCanvasLayoutData())
    deliver()

    // Truth moves with no pass in between, so a seed built from it is ahead of
    // the baseline — and of every renderer already connected.
    const page = createPage('https://example.com/late')
    selectEntities([page])

    const seed = seatSceneBootstrap(bgView!.webContents, getCanvasLayoutData())
    deliver()
    // The requester applies the seed it was handed back; by then the bus has
    // told it, and everyone else, the same thing.
    renderers[0].receive(ipcChannels.layoutUpdate, seed)

    clockSkew += 2000
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'after the seed' })
    pass()

    expect(drift()).toEqual([])
  })
})
