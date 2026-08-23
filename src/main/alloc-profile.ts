/**
 * TEMPORARY — allocation profiler for the above-view overlay renderer.
 *
 * Attributes JS allocation by call stack over a window, and brackets it with
 * Chromium's own counters so a null result is still informative: if the JS
 * heap barely moves while the process grows, the allocation is external
 * (canvas backing stores, ImageBitmaps, decoded images) and the sampling
 * profiler cannot see it by construction.
 *
 * Delete once the above-view crash (render-process-gone, exitCode 5) is fixed.
 */

import { app, type WebContents } from 'electron'
import { aboveView } from './runtime/view-refs'
import { ensurePageDebugger } from './runtime/page-debugger'

interface CallFrame {
  functionName: string
  url: string
  lineNumber: number
}

interface SamplingNode {
  callFrame: CallFrame
  selfSize: number
  children?: SamplingNode[]
}

export interface AllocSite {
  /** Innermost frame, then its callers, outermost last. */
  stack: string[]
  bytes: number
}

export interface NativeSite {
  /** Innermost native frames first. */
  stack: string[]
  bytes: number
}

export interface AllocProfileResult {
  windowMs: number
  totalSampledBytes: number
  sites: AllocSite[]
  counters: Record<string, { before: number; after: number; delta: number }>
  /** Ground truth: the OS-visible size of the renderer, which grows even when
   *  the JS heap does not. */
  rssKb: { before: number; after: number; delta: number }
  synthesizedMoves: number
  /** Native (non-JS) allocation, which is where external memory shows up. */
  nativeSites: NativeSite[]
  nativeTotalBytes: number
  note?: string
}

/** Counters worth bracketing: the first says whether this is a JS-heap story. */
const TRACKED_METRICS = [
  'JSHeapUsedSize',
  'JSHeapTotalSize',
  'Nodes',
  'JSEventListeners',
  'Documents',
  'LayoutObjects',
]

function frameLabel(frame: CallFrame): string {
  const name = frame.functionName || '(anonymous)'
  const file = frame.url ? frame.url.replace(/^.*\/src\//, 'src/').split('?')[0] : '(native)'
  return `${name} — ${file}:${frame.lineNumber + 1}`
}

/** Flattens the sampling tree, keeping each node's path back to the root. */
function collectSites(root: SamplingNode): AllocSite[] {
  const sites: AllocSite[] = []
  const walk = (node: SamplingNode, ancestors: string[]): void => {
    const label = frameLabel(node.callFrame)
    const stack = [label, ...ancestors]
    if (node.selfSize > 0) sites.push({ stack, bytes: node.selfSize })
    for (const child of node.children ?? []) walk(child, stack)
  }
  walk(root, [])
  return sites
}

/** Sites sharing an innermost frame are one allocation site, however reached. */
function mergeByLeaf(sites: AllocSite[]): AllocSite[] {
  const byLeaf = new Map<string, AllocSite>()
  for (const site of sites) {
    const existing = byLeaf.get(site.stack[0])
    if (existing) existing.bytes += site.bytes
    else byLeaf.set(site.stack[0], { stack: site.stack, bytes: site.bytes })
  }
  return [...byLeaf.values()].sort((a, b) => b.bytes - a.bytes)
}

function rssKbOf(pid: number): number {
  const metric = app.getAppMetrics().find((m) => m.pid === pid)
  return metric?.memory.workingSetSize ?? 0
}

/**
 * Drives a pointer across the overlay over CDP, so the repro does not depend
 * on a human wiggling a mouse. Moves only, never buttons — hover is the path
 * under investigation and a synthetic click would mutate the workspace.
 */
function startSynthesizedMoves(wc: WebContents, width: number, height: number): () => number {
  let count = 0
  let phase = 0
  const timer = setInterval(() => {
    phase += 1
    // A lissajous-ish sweep, so the pointer crosses pages, chrome, and empty
    // canvas rather than retracing one line.
    const x = Math.round((width / 2) * (1 + Math.sin(phase / 7)))
    const y = Math.round((height / 2) * (1 + Math.cos(phase / 11)))
    wc.debugger
      .sendCommand('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, buttons: 0 })
      .then(() => { count += 1 })
      .catch(() => {})
  }, 16)
  return () => {
    clearInterval(timer)
    return count
  }
}

interface NativeSample {
  size: number
  total: number
  stack: string[]
}

/**
 * Chromium's native sampling allocator. JS heap tooling is blind to canvas
 * backing stores, ImageBitmaps, and decoded images; this is not.
 */
async function collectNativeProfile(wc: WebContents): Promise<NativeSite[]> {
  const result = (await wc.debugger.sendCommand('Memory.getAllTimeSamplingProfile')) as {
    profile: { samples: NativeSample[] }
  }
  const byStack = new Map<string, NativeSite>()
  for (const sample of result.profile.samples ?? []) {
    // Drop the allocator plumbing every stack shares — it names no caller.
    const stack = sample.stack.filter(
      (frame) => frame && !/^(malloc|operator new|.*PartitionAlloc.*)$/.test(frame.trim()),
    )
    const key = stack.slice(0, 6).join(' < ') || '(unsymbolized)'
    const existing = byStack.get(key)
    if (existing) existing.bytes += sample.total
    else byStack.set(key, { stack: stack.slice(0, 6), bytes: sample.total })
  }
  return [...byStack.values()].sort((a, b) => b.bytes - a.bytes)
}

async function readMetrics(wc: WebContents): Promise<Record<string, number>> {
  const result = (await wc.debugger.sendCommand('Performance.getMetrics')) as {
    metrics: { name: string; value: number }[]
  }
  const out: Record<string, number> = {}
  for (const metric of result.metrics) {
    if (TRACKED_METRICS.includes(metric.name)) out[metric.name] = metric.value
  }
  return out
}

export async function runAllocProfile(
  windowMs: number,
  synthesizeMoves: boolean,
): Promise<AllocProfileResult> {
  if (!aboveView || aboveView.webContents.isDestroyed()) {
    throw new Error('above-view is not available (it may have just crashed)')
  }
  const wc = aboveView.webContents
  if (!ensurePageDebugger(wc, () => {})) {
    throw new Error('could not attach the debugger to above-view')
  }

  await wc.debugger.sendCommand('Performance.enable')
  await wc.debugger.sendCommand('HeapProfiler.enable')
  const before = await readMetrics(wc)
  const pid = wc.getOSProcessId()
  const rssBefore = rssKbOf(pid)
  await wc.debugger.sendCommand('HeapProfiler.startSampling', { samplingInterval: 32768 })
  // Sampling rate is per-allocation-bytes; 64KB keeps the profile small while
  // still catching anything allocating at megabytes per second.
  try {
    await wc.debugger.sendCommand('Memory.startSampling', { samplingInterval: 65536 })
  } catch {
    // Older builds may not expose it; the JS side of the profile still runs.
  }

  const bounds = aboveView.getBounds()
  const stopMoves = synthesizeMoves
    ? startSynthesizedMoves(wc, bounds.width, bounds.height)
    : () => 0

  await new Promise((resolve) => setTimeout(resolve, windowMs))
  const synthesizedMoves = stopMoves()
  const rssAfter = rssKbOf(pid)

  const { profile } = (await wc.debugger.sendCommand('HeapProfiler.stopSampling')) as {
    profile: { head: SamplingNode }
  }
  const after = await readMetrics(wc)
  let nativeSites: NativeSite[] = []
  try {
    nativeSites = await collectNativeProfile(wc)
    await wc.debugger.sendCommand('Memory.stopSampling')
  } catch {
    nativeSites = []
  }

  const sites = mergeByLeaf(collectSites(profile.head))
  const totalSampledBytes = sites.reduce((sum, site) => sum + site.bytes, 0)

  const counters: AllocProfileResult['counters'] = {}
  for (const name of TRACKED_METRICS) {
    const b = before[name] ?? 0
    const a = after[name] ?? 0
    counters[name] = { before: b, after: a, delta: a - b }
  }

  // The sampler reports what JS allocated, not what survived. A large sampled
  // total with a flat heap means churn the GC keeps up with; a growing heap
  // means retention.
  const heapDelta = counters.JSHeapUsedSize?.delta ?? 0
  const rssGrewMb = (rssAfter - rssBefore) / 1024
  const jsGrewMb = Math.max(totalSampledBytes, heapDelta) / (1024 * 1024)
  const note =
    rssGrewMb > 50 && jsGrewMb < rssGrewMb / 4
      ? `Renderer grew ${rssGrewMb.toFixed(0)}MB while JS accounted for only ${jsGrewMb.toFixed(0)}MB — the growth is external (canvas backing stores, ImageBitmaps, decoded images), which a JS heap sampler cannot see.`
      : undefined

  return {
    windowMs,
    totalSampledBytes,
    sites: sites.slice(0, 40),
    counters,
    rssKb: { before: rssBefore, after: rssAfter, delta: rssAfter - rssBefore },
    synthesizedMoves,
    nativeSites: nativeSites.slice(0, 25),
    nativeTotalBytes: nativeSites.reduce((sum, site) => sum + site.bytes, 0),
    note,
  }
}
