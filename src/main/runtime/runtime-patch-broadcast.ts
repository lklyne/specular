import type { WebContents } from 'electron'
import { ipcChannels } from '../../shared/ipc-contract'
import {
  applyRuntimePatch,
  type RuntimePatch,
  type RuntimePatchBatch,
} from '../../shared/runtime-patch'
import { snapshotToStore, type RuntimeStore } from '../../shared/runtime-store'
import { diffRuntimeStores } from '../../shared/runtime-store-diff'
import {
  filterPatchBatch,
  filterSceneSnapshot,
  type SceneTarget,
} from '../../shared/runtime-store-filter'
import type { LayoutUpdateData } from '../../shared/types'
import { logCrash } from '../crash-log'
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
 * heals instead of leaving stale chrome on screen. It is redundancy and
 * nothing else: a pass always sends the cells it moved, so a snapshot that
 * changes what a renderer holds is drift being corrected, never a delivery.
 * `baseline` is the store every renderer is believed to hold; every send
 * updates it, which is why a snapshot always goes to all three targets rather
 * than the one that asked.
 *
 * The baseline is always the full store. What each target receives is trimmed
 * to the slices it reads on the way out (`runtime-store-filter.ts`), so the
 * routing never leaks into main's model of the scene.
 */
const SNAPSHOT_INTERVAL_MS = 1000

let baseline: RuntimeStore | null = null
let lastSnapshotAt = 0

interface TargetView {
  target: SceneTarget
  wc: WebContents
}

function sceneTargets(): TargetView[] {
  const targets: TargetView[] = []
  if (bgView) targets.push({ target: 'canvas-bg', wc: bgView.webContents })
  if (aboveView) targets.push({ target: 'above-view', wc: aboveView.webContents })
  if (cursorOverlayWindow && !cursorOverlayWindow.isDestroyed()) {
    targets.push({ target: 'agent-layer', wc: cursorOverlayWindow.webContents })
  }
  return targets
}

/** The target a canvas renderer's own bootstrap should be trimmed to, or null
 *  when the sender is not one of them. */
export function sceneTargetFor(wc: WebContents): SceneTarget | null {
  return sceneTargets().find((view) => view.wc === wc)?.target ?? null
}

/** Send the whole scene and re-seat the baseline. Used on connect, and as the
 *  periodic reconcile baseline the patch stream rides on top of. */
export function broadcastSceneSnapshot(payload: LayoutUpdateData): void {
  fanOut(payload, true)
}

/**
 * The layout pass's fan-out: the cells this pass moved, plus a whole-scene
 * re-seat every `SNAPSHOT_INTERVAL_MS` of pass activity — the simplest cadence
 * that bounds how long any drift can live. A pass that changed nothing sends
 * nothing (unless the re-seat is due).
 */
export function broadcastSceneUpdate(payload: LayoutUpdateData): void {
  fanOut(payload, false)
}

/**
 * The patches first, then the snapshot if one is due — never one instead of
 * the other.
 *
 * A snapshot that is the only carrier for a change makes a renderer that
 * converged indistinguishable from one that has been stale for a second, which
 * is the difference the drift watchdog exists to see. So every send starts
 * from the same diff, and a snapshot is redundancy by construction: whoever
 * asked for it (a connecting renderer, the interval) gets the whole scene, and
 * everyone already connected has been told what moved either way.
 */
function fanOut(payload: LayoutUpdateData, force: boolean): void {
  const next = snapshotToStore(payload)
  const patches = baseline ? diffRuntimeStores(baseline, next) : []
  const snapshot = force || !baseline || Date.now() - lastSnapshotAt >= SNAPSHOT_INTERVAL_MS
  baseline = next
  if (patches.length > 0) {
    broadcastPatchBatch({
      patches,
      ...(payload.buildMs != null ? { buildMs: payload.buildMs } : {}),
    })
  }
  if (!snapshot) return
  lastSnapshotAt = Date.now()
  for (const { target, wc } of sceneTargets()) {
    send(target, wc, ipcChannels.layoutUpdate, filterSceneSnapshot(payload, target))
  }
}

/**
 * A canvas renderer's own seed, and the baseline catching up to it.
 *
 * The bootstrap is built from current truth, which is ahead of the baseline
 * whenever anything moved since the last pass — so handing it over without
 * telling the bus leaves main diffing against a store nobody holds. Running it
 * through the pass fan-out re-seats the baseline and brings the other
 * renderers to the same scene.
 */
export function seatSceneBootstrap(wc: WebContents, payload: LayoutUpdateData): LayoutUpdateData {
  const target = sceneTargetFor(wc)
  if (!target) return payload
  broadcastSceneUpdate(payload)
  return filterSceneSnapshot(payload, target)
}

/** Push one slice straight out, for the mutators that skip the layout pass
 *  entirely (hover, page scroll, annotation bboxes). Keeps the baseline in step
 *  so the next pass doesn't re-send what this already delivered. */
export function broadcastRuntimePatch(patch: RuntimePatch): void {
  if (baseline) baseline = applyRuntimePatch(baseline, patch)
  broadcastPatchBatch({ patches: [patch] })
}

function broadcastPatchBatch(batch: RuntimePatchBatch): void {
  for (const { target, wc } of sceneTargets()) {
    const routed = filterPatchBatch(batch, target)
    if (routed) send(target, wc, ipcChannels.runtimePatch, routed)
  }
}

function send(target: SceneTarget, wc: WebContents, channel: string, payload: unknown): void {
  recordWireBytes(target, channel, payload)
  safeSend(wc, channel, payload)
}

// TEMP instrument (plan: diffed-runtime-store) — bytes on the wire per channel
// per target, reported to errors.log every 2s. Slice routing is a claim about
// who receives what; this is what makes it checkable, and makes total
// bytes-at-rest one number to watch fall.
const wireBytes = new Map<string, number>()
let wireTimer: NodeJS.Timeout | null = null

function recordWireBytes(target: SceneTarget, channel: string, payload: unknown): void {
  const key = `${target}:${channel}`
  let bytes = 0
  try {
    bytes = JSON.stringify(payload)?.length ?? 0
  } catch {
    return
  }
  wireBytes.set(key, (wireBytes.get(key) ?? 0) + bytes)
  if (wireTimer) return
  wireTimer = setInterval(() => {
    if (wireBytes.size === 0) return
    const rows = [...wireBytes.entries()]
      .sort((a, b) => b[1] - a[1])
      .map(([name, total]) => `${name}=${total}`)
    wireBytes.clear()
    logCrash('runtime-wire-bytes', rows.join(' '))
  }, 2000)
  wireTimer.unref?.()
}
