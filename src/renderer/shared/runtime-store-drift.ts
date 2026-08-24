// drift watchdog (plan: diffed-runtime-store)
/**
 * Patches are allowed to be lossy because the next snapshot heals them. That
 * bargain is only safe if the healing actually happens, and its failure mode —
 * a leaky subscription or a lossy producer leaving stale UI — is silent. So
 * every snapshot is compared against what the patch stream accumulated, and any
 * disagreement is counted.
 *
 * A count alone names the cell and nothing else, which is one bisect short of
 * an answer: `slice:selection` could be a missed patch, a stale id, or a field
 * nobody meant to put in the store. So the first sighting of each cell also
 * reports the keys that disagree and what each side holds — enough to tell a
 * producer bug from a delivery one without another run.
 *
 * Dev-only: the comparison walks the whole store, which is exactly the O(scene)
 * work the patch bus exists to avoid.
 */

import { shareStructure } from '../../shared/layout-structural-share'
import { diffRuntimeStores } from '../../shared/runtime-store-diff'
import type { RuntimePatch } from '../../shared/runtime-patch'
import type { RuntimeStore } from '../../shared/runtime-store'

const REPORT_INTERVAL_MS = 2000
/** Keys per cell, how deep a differing key is chased, and cells detailed per
 *  snapshot — a scene-wide drift is one bug, not two hundred log lines. */
const MAX_KEYS = 5
const MAX_DEPTH = 2
const MAX_CELLS = 6

/** The one gate for every dev-only consistency check in the renderer — this
 *  file's snapshot diff and the projection drift assertion both cost O(scene)
 *  work that has no place in a shipped build. */
const driftWatchdogEnabled =
  ((import.meta as unknown as { env?: { DEV?: boolean } }).env?.DEV ?? false) === true

let mismatches = 0
let snapshots = 0
let firstReported = false
let timer: ReturnType<typeof setInterval> | null = null
const detailed = new Set<string>()

function equal(a: unknown, b: unknown): boolean {
  return Object.is(shareStructure(a, b), a)
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function summarize(value: unknown): string {
  if (value === undefined) return '∅'
  if (value === null) return 'null'
  if (Array.isArray(value)) {
    const short = value.length <= 4 && value.every((item) => typeof item === 'string')
    return short ? `[${value.map((item) => summarize(item)).join(',')}]` : `[${value.length}]`
  }
  if (isPlainObject(value)) {
    const keys = Object.keys(value)
    return `{${keys.slice(0, 4).join(',')}${keys.length > 4 ? ',…' : ''}}`
  }
  if (typeof value === 'string') {
    return JSON.stringify(value.length > 16 ? `${value.slice(0, 16)}…` : value)
  }
  return String(value)
}

/**
 * The keys where two values disagree, chased far enough to name a field.
 *
 * Walking arbitrary held values is best-effort — a throwing accessor, a
 * pathological shape — so it is fenced off from the report in `describeCell`.
 */
function describeDelta(before: unknown, after: unknown, path = '', depth = 0): string[] {
  if (depth < MAX_DEPTH && isPlainObject(before) && isPlainObject(after)) {
    const keys = [...new Set([...Object.keys(before), ...Object.keys(after)])]
    const rows: string[] = []
    for (const key of keys) {
      if (equal(before[key], after[key])) continue
      rows.push(...describeDelta(before[key], after[key], path ? `${path}.${key}` : key, depth + 1))
      if (rows.length >= MAX_KEYS) break
    }
    if (rows.length > 0) return rows
  }
  return [`${path || 'value'}=${summarize(before)}→${summarize(after)}`]
}

/**
 * The keys line, or why there isn't one.
 *
 * The cell name and the counters are the guard; the keys are the convenience.
 * So a walk that throws costs its own detail and nothing else — not the rest of
 * the cells, not the periodic counter, and not the snapshot the caller is in
 * the middle of applying.
 */
function describeCell(before: unknown, after: unknown): string[] {
  try {
    return describeDelta(before, after)
  } catch (error) {
    return [`describe-failed=${error instanceof Error ? error.message : String(error)}`]
  }
}

function cellName(patch: RuntimePatch): string {
  return patch.kind === 'slice' ? `slice:${patch.slice}` : `entity:${patch.id}`
}

function heldValue(store: RuntimeStore, patch: RuntimePatch): unknown {
  return patch.kind === 'slice' ? store.slices[patch.slice] : store.entities[patch.id]
}

function incomingValue(patch: RuntimePatch): unknown {
  return patch.kind === 'slice' ? patch.value : patch.entity
}

/** Compare the patch-accumulated store against the snapshot that just landed. */
export function reportReconcileDrift(accumulated: RuntimeStore, snapshot: RuntimeStore): void {
  if (!driftWatchdogEnabled) return
  snapshots += 1
  const drifted = diffRuntimeStores(accumulated, snapshot)
  if (drifted.length === 0) return
  mismatches += drifted.length
  if (!firstReported) {
    firstReported = true
    const detail = drifted.slice(0, 8).map(cellName).join(' ')
    console.warn(`[drift-watchdog] first drift after ${snapshots} snapshots: ${detail}`)
  }
  // Armed before the per-cell detail, so the running count — the part that
  // survives a session — never depends on the detail walk finishing.
  if (!timer) {
    timer = setInterval(() => {
      if (mismatches === 0) return
      console.warn(`[drift-watchdog] ${mismatches} drifted cells over ${snapshots} snapshots`)
      mismatches = 0
      snapshots = 0
    }, REPORT_INTERVAL_MS)
    ;(timer as { unref?: () => void }).unref?.()
  }
  let described = 0
  for (const patch of drifted) {
    if (described >= MAX_CELLS) break
    const name = cellName(patch)
    if (detailed.has(name)) continue
    detailed.add(name)
    described += 1
    const rows = describeCell(heldValue(accumulated, patch), incomingValue(patch))
    const shown = rows.slice(0, MAX_KEYS).join(' ')
    const more = rows.length > MAX_KEYS ? ` +${rows.length - MAX_KEYS} more` : ''
    console.warn(`[drift-watchdog] ${name} held→snapshot: ${shown}${more}`)
  }
}
