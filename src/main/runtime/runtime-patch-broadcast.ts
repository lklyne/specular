import type { WebContents } from 'electron'
import { ipcChannels } from '../../shared/ipc-contract'
import {
  applyRuntimePatch,
  type RuntimePatch,
  type RuntimePatchBatch,
} from '../../shared/runtime-patch'
import { snapshotToStore, type RuntimeStore } from '../../shared/runtime-store'
import { diffRuntimeStores } from '../../shared/runtime-store-diff'
import type { LayoutUpdateData } from '../../shared/types'
import { aboveView, bgView, cursorOverlayWindow } from './view-refs'
import { safeSend } from './safe-send'

/**
 * The scene bus: one full-snapshot channel and one patch channel, over the
 * same three canvas renderers.
 *
 * A layout pass rebuilds the whole scene and re-serializes it for every
 * consumer, so its cost is set by scene size rather than by what moved. Diffing
 * the rebuild against what the renderers already hold turns a pass into the
 * handful of cells that actually changed.
 *
 * `layoutUpdate` never goes away. It is what a renderer gets on connect, and
 * what it gets periodically after that, so a dropped or mis-applied patch
 * heals instead of leaving stale chrome on screen. `baseline` is the store
 * every renderer is believed to hold; every send updates it, which is why a
 * snapshot always goes to all three targets rather than the one that asked.
 */
const SNAPSHOT_INTERVAL_MS = 1000

let baseline: RuntimeStore | null = null
let lastSnapshotAt = 0

function sceneTargets(): WebContents[] {
  const targets: WebContents[] = []
  if (bgView) targets.push(bgView.webContents)
  if (aboveView) targets.push(aboveView.webContents)
  if (cursorOverlayWindow && !cursorOverlayWindow.isDestroyed()) {
    targets.push(cursorOverlayWindow.webContents)
  }
  return targets
}

/** Send the whole scene and re-seat the baseline. Used on connect, and as the
 *  periodic reconcile baseline `broadcastSceneUpdate` falls back to. */
export function broadcastSceneSnapshot(payload: LayoutUpdateData): void {
  baseline = snapshotToStore(payload)
  lastSnapshotAt = Date.now()
  for (const wc of sceneTargets()) safeSend(wc, ipcChannels.layoutUpdate, payload)
}

/**
 * The layout pass's fan-out. Sends patches, except every `SNAPSHOT_INTERVAL_MS`
 * of pass activity, when it sends a snapshot instead — the simplest cadence
 * that bounds how long any drift can live. A pass that changed nothing sends
 * nothing.
 */
export function broadcastSceneUpdate(payload: LayoutUpdateData): void {
  if (!baseline || Date.now() - lastSnapshotAt >= SNAPSHOT_INTERVAL_MS) {
    broadcastSceneSnapshot(payload)
    return
  }
  const next = snapshotToStore(payload)
  const patches = diffRuntimeStores(baseline, next)
  baseline = next
  if (patches.length === 0) return
  broadcastPatchBatch({
    patches,
    ...(payload.buildMs != null ? { buildMs: payload.buildMs } : {}),
  })
}

/** Push one slice straight out, for the mutators that skip the layout pass
 *  entirely (hover). Keeps the baseline in step so the next pass doesn't
 *  re-send what this already delivered. */
export function broadcastRuntimePatch(patch: RuntimePatch): void {
  if (baseline) baseline = applyRuntimePatch(baseline, patch)
  broadcastPatchBatch({ patches: [patch] })
}

function broadcastPatchBatch(batch: RuntimePatchBatch): void {
  for (const wc of sceneTargets()) safeSend(wc, ipcChannels.runtimePatch, batch)
}
