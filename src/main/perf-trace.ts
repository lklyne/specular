/**
 * Perf trace — records a Chromium tracing session across every process
 * (browser, GPU/Viz, all page renderers) so pan/zoom jank can be attributed
 * to browser CPU, renderer raster, surface sync, or display compositing.
 *
 * Output is a Chrome-JSON trace file; open it at https://ui.perfetto.dev.
 * Category rationale and how to read the result: docs/pan-zoom-perf-unknowns.md §2.G.
 */
import { app, contentTracing, shell } from 'electron'
import path from 'path'

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

let recording = false
let autoStopTimer: NodeJS.Timeout | null = null
let stateListener: (() => void) | null = null

/** Register a callback fired whenever recording starts or stops (including
 * the auto-stop), so UI like the app menu can refresh its label. */
export function setPerfTraceStateListener(listener: () => void): void {
  stateListener = listener
}

export function isPerfTraceRecording(): boolean {
  return recording
}

export async function togglePerfTrace(): Promise<void> {
  if (recording) {
    await stopPerfTrace()
  } else {
    await startPerfTrace()
  }
}

async function startPerfTrace(): Promise<void> {
  if (recording) return
  await contentTracing.startRecording({
    included_categories: TRACE_CATEGORIES,
    recording_mode: 'record-until-full',
    trace_buffer_size_in_kb: TRACE_BUFFER_KB,
  })
  recording = true
  autoStopTimer = setTimeout(() => {
    void stopPerfTrace()
  }, MAX_TRACE_MS)
  stateListener?.()
}

async function stopPerfTrace(): Promise<void> {
  if (!recording) return
  if (autoStopTimer) {
    clearTimeout(autoStopTimer)
    autoStopTimer = null
  }
  recording = false
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  const outPath = path.join(app.getPath('logs'), `specular-trace-${stamp}.json`)
  const savedPath = await contentTracing.stopRecording(outPath)
  stateListener?.()
  shell.showItemInFolder(savedPath)
}
