import type { ReactNode } from 'react'
import type { PerfTraceState } from '../../shared/electron-api/debug'
import type { PanZoomPerfTestState } from '../../shared/pan-zoom-perf-test'
import { formatClock } from './format'

const RECORD_MAX_SECONDS = 30
const ACTIVE_BUTTON_CLASS =
  'flex items-center gap-2 rounded border border-red-400/60 bg-red-500/10 px-2 py-1 text-[11px] font-medium text-red-600 hover:bg-red-500/20 disabled:opacity-50 dark:border-red-500/50 dark:text-red-400'
const IDLE_BUTTON_CLASS =
  'flex items-center gap-2 rounded border border-zinc-300 px-2 py-1 text-[11px] font-medium hover:bg-zinc-100 disabled:opacity-50 dark:border-zinc-700 dark:hover:bg-zinc-800'

export function PerformanceHeader({
  traceState,
  panZoomState,
  elapsedSeconds,
  onToggle,
  onPanZoomTest,
  onRefresh,
}: {
  traceState: PerfTraceState
  panZoomState: PanZoomPerfTestState
  elapsedSeconds: number
  onToggle: () => void
  onPanZoomTest: () => void
  onRefresh: () => void
}) {
  return (
    <div className="shrink-0 border-b border-[var(--surface-popover-border)] px-4 py-3">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[13px] font-semibold">Performance</div>
        <TraceControls
          traceState={traceState}
          panZoomRunning={panZoomState.running}
          elapsedSeconds={elapsedSeconds}
          onToggle={onToggle}
          onRefresh={onRefresh}
        />
      </div>
      <div className="mt-1 text-[10px] leading-snug text-zinc-500 dark:text-zinc-400">
        Traces are saved to the app logs folder; recording auto-stops after{' '}
        {RECORD_MAX_SECONDS}s.
      </div>
      <PanZoomTestControl
        traceState={traceState}
        panZoomState={panZoomState}
        onToggle={onPanZoomTest}
      />
    </div>
  )
}

function TraceControls({
  traceState,
  panZoomRunning,
  elapsedSeconds,
  onToggle,
  onRefresh,
}: {
  traceState: PerfTraceState
  panZoomRunning: boolean
  elapsedSeconds: number
  onToggle: () => void
  onRefresh: () => void
}) {
  const traceBusy = traceState.status === 'starting' || traceState.status === 'stopping'
  const active = traceState.status === 'recording' || traceState.status === 'stopping'
  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={onRefresh}
        className="rounded border border-zinc-300 px-2 py-1 text-[11px] hover:bg-zinc-100 dark:border-zinc-700 dark:hover:bg-zinc-800"
      >
        Refresh
      </button>
      <button
        type="button"
        onClick={onToggle}
        disabled={traceBusy || panZoomRunning}
        className={active ? ACTIVE_BUTTON_CLASS : IDLE_BUTTON_CLASS}
      >
        {traceButtonContent(traceState, elapsedSeconds)}
      </button>
    </div>
  )
}

function traceButtonContent(traceState: PerfTraceState, elapsedSeconds: number): ReactNode {
  if (traceState.status === 'stopping') return <span>Saving…</span>
  if (traceState.status === 'starting') return <span>Starting…</span>
  if (traceState.status !== 'recording') return <span>Record trace</span>
  return (
    <>
      <span className="h-1.5 w-1.5 shrink-0 animate-pulse rounded-full bg-red-500" />
      <span className="tabular-nums">{formatClock(elapsedSeconds)}</span>
      <span>Stop</span>
    </>
  )
}

function PanZoomTestControl({
  traceState,
  panZoomState,
  onToggle,
}: {
  traceState: PerfTraceState
  panZoomState: PanZoomPerfTestState
  onToggle: () => void
}) {
  const disabled = panZoomState.stopping ||
    (!panZoomState.running && traceState.status !== 'idle')
  const label = panZoomState.stopping
    ? 'Stopping…'
    : panZoomState.running
      ? 'Stop test'
      : 'Run test'
  return (
    <div className="mt-3 flex items-center justify-between gap-3 rounded border border-zinc-200 p-2 dark:border-zinc-800">
      <div className="min-w-0">
        <div className="text-[11px] font-medium">Pan/zoom test</div>
        <div className="truncate text-[10px] text-zinc-500 dark:text-zinc-400">
          {panZoomState.running
            ? panZoomState.phase ?? 'Running'
            : 'Slow and fast pan, zoom, diagonal, and combined gestures'}
        </div>
      </div>
      <button
        type="button"
        onClick={onToggle}
        disabled={disabled}
        className={panZoomState.running ? ACTIVE_BUTTON_CLASS : IDLE_BUTTON_CLASS}
      >
        {label}
      </button>
    </div>
  )
}
