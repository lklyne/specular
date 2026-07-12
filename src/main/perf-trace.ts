/**
 * Perf trace — records a Chromium tracing session across every process
 * (browser, GPU/Viz, all page renderers) so pan/zoom jank can be attributed
 * to browser CPU, renderer raster, surface sync, or display compositing.
 *
 * Output is a Chrome-JSON trace file; open it at https://ui.perfetto.dev.
 * Category rationale and how to read the result: docs/pan-zoom-perf-unknowns.md §2.G.
 */
import { app, contentTracing, shell } from 'electron'
import { promises as fs } from 'fs'
import path from 'path'
import { ipcChannels } from '../shared/ipc-contract'
import { extractTraceEvents, summarizeTraceEvents, type TraceSummary } from '../shared/trace-summary'
import type { PerfTraceFileEntry, PerfTraceState } from '../shared/electron-api/debug'
import { getDebugWebContents } from './debug-window'

const TRACE_CATEGORIES = [
  'viz',
  'cc',
  'gpu',
  'blink',
  'benchmark',
  'toplevel',
  'toplevel.flow',
  'input',
  'latency',
  'latencyInfo',
  'sequence_manager',
  'graphics.pipeline',
  'electron',
  'disabled-by-default-devtools.timeline.frame',
]

/** Hard stop so a forgotten recording can't run away — these categories
 * produce on the order of tens of MB per second across 20+ processes. */
const MAX_TRACE_MS = 30_000

const TRACE_BUFFER_KB = 300_000

/** Matches saved trace files, excluding the cached `.summary.json` sidecars. */
const TRACE_FILE_RE = /^specular-trace-.*\.json$/
/** Filenames accepted from IPC/HTTP callers — no path separators, no traversal. */
const SAFE_TRACE_FILENAME_RE = /^specular-trace-[A-Za-z0-9-]+\.json$/
const SUMMARY_SUFFIX = '.summary.json'
/** Above this size, parsing synchronously on main is too disruptive to attempt. */
const MAX_SUMMARIZABLE_BYTES = 500 * 1024 * 1024

let status: PerfTraceState['status'] = 'idle'
let startedAt: number | null = null
let autoStopTimer: NodeJS.Timeout | null = null
let revealOnAutoStop = true
let stateListener: (() => void) | null = null
let stopPromise: Promise<string | null> | null = null

/** Register a callback fired whenever recording starts or stops (including
 * the auto-stop), so UI like the app menu can refresh its label. */
export function setPerfTraceStateListener(listener: () => void): void {
  stateListener = listener
}

export function isPerfTraceRecording(): boolean {
  return status !== 'idle'
}

export function getPerfTraceState(): PerfTraceState {
  return { recording: status === 'recording', status, startedAt }
}

function notifyStateChange(): void {
  stateListener?.()
  // Returns null when the debug window isn't open (or has been destroyed);
  // nothing to broadcast to in that case.
  getDebugWebContents()?.send(ipcChannels.debugPerfTraceStateChanged, getPerfTraceState())
}

export async function togglePerfTrace(): Promise<void> {
  if (status === 'recording') {
    await stopPerfTrace()
  } else if (status === 'idle') {
    await startPerfTrace()
  }
}

export async function startPerfTrace(options: { revealOnAutoStop?: boolean } = {}): Promise<void> {
  if (status !== 'idle') return
  status = 'starting'
  notifyStateChange()
  try {
    await contentTracing.startRecording({
      included_categories: TRACE_CATEGORIES,
      recording_mode: 'record-until-full',
      trace_buffer_size_in_kb: TRACE_BUFFER_KB,
    })
    status = 'recording'
    startedAt = Date.now()
    revealOnAutoStop = options.revealOnAutoStop !== false
    autoStopTimer = setTimeout(() => {
      void stopPerfTrace({ reveal: revealOnAutoStop })
    }, MAX_TRACE_MS)
    notifyStateChange()
  } catch (error) {
    status = 'idle'
    startedAt = null
    notifyStateChange()
    throw error
  }
}

/** Stops the active recording and returns the saved trace's absolute path.
 * Interactive callers reveal the artifact by default; headless callers can
 * suppress Finder so collecting agent diagnostics does not steal focus. */
export async function stopPerfTrace(options: { reveal?: boolean } = {}): Promise<string | null> {
  if (status === 'stopping') return stopPromise
  if (status !== 'recording') return null
  if (autoStopTimer) {
    clearTimeout(autoStopTimer)
    autoStopTimer = null
  }
  status = 'stopping'
  notifyStateChange()
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  const outPath = path.join(app.getPath('logs'), `specular-trace-${stamp}.json`)
  stopPromise = (async () => {
    try {
      const savedPath = await contentTracing.stopRecording(outPath)
      if (options.reveal !== false) shell.showItemInFolder(savedPath)
      return savedPath
    } finally {
      status = 'idle'
      startedAt = null
      stopPromise = null
      notifyStateChange()
    }
  })()
  return stopPromise
}

export async function listPerfTraces(): Promise<PerfTraceFileEntry[]> {
  const dir = app.getPath('logs')
  let names: string[]
  try {
    names = await fs.readdir(dir)
  } catch {
    return []
  }

  const traceNames = names.filter((n) => TRACE_FILE_RE.test(n) && !n.endsWith(SUMMARY_SUFFIX))
  const entries = await Promise.all(
    traceNames.map(async (fileName): Promise<PerfTraceFileEntry> => {
      const stat = await fs.stat(path.join(dir, fileName))
      const summaryPath = path.join(dir, `${path.basename(fileName, '.json')}${SUMMARY_SUFFIX}`)
      const hasSummary = await fs
        .access(summaryPath)
        .then(() => true)
        .catch(() => false)
      return { fileName, sizeBytes: stat.size, modifiedAt: stat.mtimeMs, hasSummary }
    }),
  )

  entries.sort((a, b) => b.modifiedAt - a.modifiedAt)
  return entries
}

/** Analyzes a saved trace on demand, caching the result to `<base>.summary.json`
 * beside it so repeat requests are instant. `fileName` is caller-controlled (IPC
 * or HTTP), so it's validated against `SAFE_TRACE_FILENAME_RE` before touching
 * disk — no path separators, no `..`, no escaping the logs directory. */
export async function getTraceSummary(fileName: string): Promise<TraceSummary | null> {
  if (!SAFE_TRACE_FILENAME_RE.test(fileName)) return null

  const dir = app.getPath('logs')
  const tracePath = path.join(dir, fileName)
  const summaryPath = path.join(dir, `${path.basename(fileName, '.json')}${SUMMARY_SUFFIX}`)

  try {
    const cached = await fs.readFile(summaryPath, 'utf8')
    return JSON.parse(cached) as TraceSummary
  } catch {
    // No cached summary yet — fall through and compute one.
  }

  let stat: Awaited<ReturnType<typeof fs.stat>>
  try {
    stat = await fs.stat(tracePath)
  } catch {
    return null
  }
  if (stat.size > MAX_SUMMARIZABLE_BYTES) return null

  // Trace files are opened rarely and only by this debug tool, so a blocking
  // JSON.parse of a large file (stalling main for the duration) is an
  // acceptable tradeoff against the complexity of a streaming parser.
  const raw = await fs.readFile(tracePath, 'utf8')
  const events = extractTraceEvents(JSON.parse(raw))
  const summary = summarizeTraceEvents(events)
  await fs.writeFile(summaryPath, JSON.stringify(summary), 'utf8')
  return summary
}

export function revealTrace(fileName: string): void {
  if (!SAFE_TRACE_FILENAME_RE.test(fileName)) return
  shell.showItemInFolder(path.join(app.getPath('logs'), fileName))
}
