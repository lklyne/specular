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

import type { WebContents } from 'electron'
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

export interface AllocProfileResult {
  windowMs: number
  totalSampledBytes: number
  sites: AllocSite[]
  counters: Record<string, { before: number; after: number; delta: number }>
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

export async function runAllocProfile(windowMs: number): Promise<AllocProfileResult> {
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
  await wc.debugger.sendCommand('HeapProfiler.startSampling', { samplingInterval: 32768 })

  await new Promise((resolve) => setTimeout(resolve, windowMs))

  const { profile } = (await wc.debugger.sendCommand('HeapProfiler.stopSampling')) as {
    profile: { head: SamplingNode }
  }
  const after = await readMetrics(wc)

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
  const note =
    totalSampledBytes < 8 * 1024 * 1024 && heapDelta < 8 * 1024 * 1024
      ? 'JS allocation is small over this window — the growth is likely external (canvas backing stores, ImageBitmaps, decoded images), which this profiler cannot see.'
      : undefined

  return { windowMs, totalSampledBytes, sites: sites.slice(0, 40), counters, note }
}
