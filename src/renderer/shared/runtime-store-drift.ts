// drift watchdog (plan: diffed-runtime-store)
/**
 * Patches are allowed to be lossy because the next snapshot heals them. That
 * bargain is only safe if the healing actually happens, and its failure mode —
 * a leaky subscription or a lossy producer leaving stale UI — is silent. So
 * every snapshot is compared against what the patch stream accumulated, and any
 * disagreement is counted.
 *
 * Dev-only: the comparison walks the whole store, which is exactly the O(scene)
 * work the patch bus exists to avoid.
 */

import { diffRuntimeStores } from '../../shared/runtime-store-diff'
import type { RuntimeStore } from '../../shared/runtime-store'

const REPORT_INTERVAL_MS = 2000

export const driftWatchdogEnabled =
  ((import.meta as unknown as { env?: { DEV?: boolean } }).env?.DEV ?? false) === true

let mismatches = 0
let snapshots = 0
let firstReported = false
let timer: ReturnType<typeof setInterval> | null = null

/** Compare the patch-accumulated store against the snapshot that just landed. */
export function reportReconcileDrift(accumulated: RuntimeStore, snapshot: RuntimeStore): void {
  if (!driftWatchdogEnabled) return
  snapshots += 1
  const drifted = diffRuntimeStores(accumulated, snapshot)
  if (drifted.length === 0) return
  mismatches += drifted.length
  if (!firstReported) {
    firstReported = true
    const detail = drifted
      .slice(0, 8)
      .map((patch) => (patch.kind === 'slice' ? `slice:${patch.slice}` : `entity:${patch.id}`))
      .join(' ')
    console.warn(`[drift-watchdog] first drift after ${snapshots} snapshots: ${detail}`)
  }
  if (timer) return
  timer = setInterval(() => {
    if (mismatches === 0) return
    console.warn(`[drift-watchdog] ${mismatches} drifted cells over ${snapshots} snapshots`)
    mismatches = 0
    snapshots = 0
  }, REPORT_INTERVAL_MS)
  ;(timer as { unref?: () => void }).unref?.()
}
