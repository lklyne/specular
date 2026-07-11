/**
 * Trace summary — pure aggregation over Chrome-JSON trace events (the format
 * `contentTracing.stopRecording` writes). Produces the small, chartable shape
 * the debug window and the /perf HTTP routes serve, so a human or an agent can
 * read where a gesture's time went without opening the full trace in Perfetto.
 *
 * All timestamps/durations in the input are microseconds; the summary is ms.
 */

export interface RawTraceEvent {
  ph?: string
  name?: string
  cat?: string
  pid?: number
  tid?: number
  ts?: number
  dur?: number
  args?: { name?: string } & Record<string, unknown>
}

export interface TraceThreadSummary {
  pid: number
  tid: number
  process: string
  thread: string
  /** Self time: interval durations minus time spent in nested intervals. */
  busyMs: number
  eventCount: number
}

export interface TraceEventStat {
  name: string
  totalMs: number
  count: number
}

export interface TraceTimelineSeries {
  key: string
  label: string
  /** Self-time ms per bucket, length === bucketCount. */
  busyMs: number[]
}

export interface TraceMarkerStat {
  label: string
  count: number
  totalMs: number
}

export interface TraceSummary {
  durationMs: number
  eventCount: number
  bucketMs: number
  bucketCount: number
  threads: TraceThreadSummary[]
  topEvents: TraceEventStat[]
  timeline: TraceTimelineSeries[]
  markers: TraceMarkerStat[]
}

/** Accepts either the `{ traceEvents: [...] }` envelope or a bare array. */
export function extractTraceEvents(parsed: unknown): RawTraceEvent[] {
  if (Array.isArray(parsed)) return parsed as RawTraceEvent[]
  if (
    parsed !== null &&
    typeof parsed === 'object' &&
    Array.isArray((parsed as { traceEvents?: unknown }).traceEvents)
  ) {
    return (parsed as { traceEvents: RawTraceEvent[] }).traceEvents
  }
  return []
}

/** Signals worth counting even when they don't top the duration table —
 * each maps a hypothesis from docs/pan-zoom-perf-unknowns.md to a number. */
const MARKER_PATTERNS: { label: string; test: (name: string) => boolean }[] = [
  { label: 'Layout / style recalc', test: (n) => /Layout|UpdateLayoutTree|StyleAndLayout/.test(n) },
  { label: 'Raster tasks', test: (n) => /RasterTask|RasterizerTask/.test(n) },
  { label: 'Display draw & swap', test: (n) => n.includes('Display::DrawAndSwap') },
  { label: 'Surface aggregation', test: (n) => /SurfaceAggregat/.test(n) },
  { label: 'Compositor commits', test: (n) => /ProxyMain::.*Commit|LayerTreeHost.*Commit|^Commit$/.test(n) },
  { label: 'Device emulation', test: (n) => /DeviceEmulation|EnableDeviceEmulation/.test(n) },
]

interface Interval {
  start: number
  end: number
  name: string
}

const TOP_THREADS = 20
const TOP_EVENTS = 20
const TIMELINE_SERIES = 6
const MAX_BUCKETS = 400
const MIN_BUCKET_US = 100_000

export function summarizeTraceEvents(events: RawTraceEvent[]): TraceSummary {
  const processNames = new Map<number, string>()
  const threadNames = new Map<string, string>()
  const intervalsByThread = new Map<string, Interval[]>()
  const eventStats = new Map<string, { totalMs: number; count: number }>()
  const markers = MARKER_PATTERNS.map((p) => ({ label: p.label, count: 0, totalMs: 0 }))

  let minTs = Infinity
  let maxTs = -Infinity
  let intervalCount = 0

  // Pass 1: names, complete ('X') intervals, and B/E pairing state.
  const openBeginStacks = new Map<string, RawTraceEvent[]>()
  const sortedByTs = [...events].sort((a, b) => (a.ts ?? 0) - (b.ts ?? 0))
  for (const ev of sortedByTs) {
    if (ev.ph === 'M') {
      if (ev.name === 'process_name' && ev.pid !== undefined && ev.args?.name) {
        processNames.set(ev.pid, String(ev.args.name))
      } else if (ev.name === 'thread_name' && ev.pid !== undefined && ev.tid !== undefined && ev.args?.name) {
        threadNames.set(`${ev.pid}:${ev.tid}`, String(ev.args.name))
      }
      continue
    }
    if (ev.pid === undefined || ev.tid === undefined || ev.ts === undefined) continue
    const threadKey = `${ev.pid}:${ev.tid}`

    if (ev.ph === 'X') {
      const dur = ev.dur ?? 0
      addInterval(intervalsByThread, threadKey, { start: ev.ts, end: ev.ts + dur, name: ev.name ?? '(unnamed)' })
      intervalCount++
      if (ev.ts < minTs) minTs = ev.ts
      if (ev.ts + dur > maxTs) maxTs = ev.ts + dur
    } else if (ev.ph === 'B') {
      let stack = openBeginStacks.get(threadKey)
      if (!stack) {
        stack = []
        openBeginStacks.set(threadKey, stack)
      }
      stack.push(ev)
    } else if (ev.ph === 'E') {
      const begin = openBeginStacks.get(threadKey)?.pop()
      if (begin?.ts !== undefined && begin.ts <= ev.ts) {
        addInterval(intervalsByThread, threadKey, { start: begin.ts, end: ev.ts, name: begin.name ?? '(unnamed)' })
        intervalCount++
        if (begin.ts < minTs) minTs = begin.ts
        if (ev.ts > maxTs) maxTs = ev.ts
      }
    }
  }

  if (intervalCount === 0 || !isFinite(minTs)) {
    return {
      durationMs: 0,
      eventCount: events.length,
      bucketMs: 100,
      bucketCount: 0,
      threads: [],
      topEvents: [],
      timeline: [],
      markers: markers.filter((m) => m.count > 0),
    }
  }

  const durationUs = maxTs - minTs
  const bucketUs = Math.max(
    MIN_BUCKET_US,
    Math.ceil(durationUs / MAX_BUCKETS / MIN_BUCKET_US) * MIN_BUCKET_US,
  )
  const bucketCount = Math.max(1, Math.ceil(durationUs / bucketUs))

  // Pass 2 (per thread): self time via containment sweep, bucketed timeline,
  // event-name totals, marker counts.
  const threadSummaries: TraceThreadSummary[] = []
  const threadBuckets = new Map<string, number[]>()
  for (const [threadKey, intervals] of intervalsByThread) {
    intervals.sort((a, b) => a.start - b.start || b.end - a.end)
    const buckets = new Array<number>(bucketCount).fill(0)
    let busyUs = 0

    // Containment sweep: an interval's self time is its duration minus the
    // durations of intervals nested inside it.
    const stack: { end: number; childUs: number }[] = []
    for (const iv of intervals) {
      const dur = Math.max(0, iv.end - iv.start)

      while (stack.length > 0 && stack[stack.length - 1].end <= iv.start) {
        stack.pop()
      }
      if (stack.length > 0) {
        stack[stack.length - 1].childUs += dur
      }
      stack.push({ end: iv.end, childUs: 0 })

      // Defer self-time accounting until the interval closes — walk the whole
      // list once more instead by accounting at push time using a second pass.
      // (Handled below via closedIntervals.)
      closedIntervals.push({ iv, frame: stack[stack.length - 1] })

      const stats = eventStats.get(iv.name)
      if (stats) {
        stats.totalMs += dur / 1000
        stats.count++
      } else {
        eventStats.set(iv.name, { totalMs: dur / 1000, count: 1 })
      }
      for (let i = 0; i < MARKER_PATTERNS.length; i++) {
        if (MARKER_PATTERNS[i].test(iv.name)) {
          markers[i].count++
          markers[i].totalMs += dur / 1000
        }
      }
    }

    // Now every frame's childUs is final; compute self time + buckets.
    for (const { iv, frame } of closedIntervals) {
      const dur = Math.max(0, iv.end - iv.start)
      const selfUs = Math.max(0, dur - frame.childUs)
      busyUs += selfUs
      if (dur > 0 && selfUs > 0) {
        const density = selfUs / dur
        const firstBucket = Math.floor((iv.start - minTs) / bucketUs)
        const lastBucket = Math.min(bucketCount - 1, Math.floor((iv.end - minTs) / bucketUs))
        for (let b = Math.max(0, firstBucket); b <= lastBucket; b++) {
          const bucketStart = minTs + b * bucketUs
          const overlap = Math.min(iv.end, bucketStart + bucketUs) - Math.max(iv.start, bucketStart)
          if (overlap > 0) buckets[b] += (overlap * density) / 1000
        }
      }
    }
    closedIntervals.length = 0

    const [pidStr, tidStr] = threadKey.split(':')
    const pid = Number(pidStr)
    const tid = Number(tidStr)
    threadSummaries.push({
      pid,
      tid,
      process: processNames.get(pid) ?? `pid ${pid}`,
      thread: threadNames.get(threadKey) ?? `tid ${tid}`,
      busyMs: busyUs / 1000,
      eventCount: intervals.length,
    })
    threadBuckets.set(threadKey, buckets)
  }

  threadSummaries.sort((a, b) => b.busyMs - a.busyMs)
  const topThreads = threadSummaries.slice(0, TOP_THREADS)

  const topEvents = [...eventStats.entries()]
    .map(([name, s]) => ({ name, totalMs: s.totalMs, count: s.count }))
    .sort((a, b) => b.totalMs - a.totalMs)
    .slice(0, TOP_EVENTS)

  const timeline: TraceTimelineSeries[] = topThreads.slice(0, TIMELINE_SERIES).map((t) => ({
    key: `${t.pid}:${t.tid}`,
    label: `${t.process} — ${t.thread}`,
    busyMs: (threadBuckets.get(`${t.pid}:${t.tid}`) ?? []).map((v) => round2(v)),
  }))

  return {
    durationMs: round2(durationUs / 1000),
    eventCount: events.length,
    bucketMs: bucketUs / 1000,
    bucketCount,
    threads: topThreads.map((t) => ({ ...t, busyMs: round2(t.busyMs) })),
    topEvents: topEvents.map((e) => ({ ...e, totalMs: round2(e.totalMs) })),
    timeline,
    markers: markers
      .filter((m) => m.count > 0)
      .map((m) => ({ ...m, totalMs: round2(m.totalMs) })),
  }
}

// Scratch shared between the two per-thread passes; module-level to avoid
// re-allocating per thread, always drained before reuse.
const closedIntervals: { iv: Interval; frame: { end: number; childUs: number } }[] = []

function addInterval(map: Map<string, Interval[]>, key: string, iv: Interval): void {
  const list = map.get(key)
  if (list) list.push(iv)
  else map.set(key, [iv])
}

function round2(n: number): number {
  return Math.round(n * 100) / 100
}
