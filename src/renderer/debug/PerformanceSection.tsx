/**
 * Performance debug panel — record a Chromium trace, list saved trace files,
 * and analyze one into charts below. Traces are Chrome-JSON; the same files
 * also open directly in https://ui.perfetto.dev for deeper inspection.
 */

import { useCallback, useEffect, useState, type ReactNode } from 'react'
import type {
  DebugElectronAPI,
  PerfTraceFileEntry,
  PerfTraceState,
} from '../../shared/electron-api/debug'
import type { TraceSummary } from '../../shared/trace-summary'
import type { PanZoomPerfTestState } from '../../shared/pan-zoom-perf-test'
import { HorizontalBarChart, TimelineChart } from './charts'
import { formatClock, formatMs, formatModified, humanBytes } from './format'

const RECORD_MAX_SECONDS = 30
const MAX_BARS = 12
const MAX_MARKER_LABEL = 40

export function PerformanceSection({ api }: { api: DebugElectronAPI }) {
  const [traceState, setTraceState] = useState<PerfTraceState>({
    recording: false,
    status: 'idle',
    startedAt: null,
  })
  const [panZoomState, setPanZoomState] = useState<PanZoomPerfTestState>({
    running: false,
    phase: null,
    startedAt: null,
  })
  const [actionError, setActionError] = useState<string | null>(null)
  const [elapsedSeconds, setElapsedSeconds] = useState(0)
  const [traces, setTraces] = useState<PerfTraceFileEntry[]>([])
  const [loadingList, setLoadingList] = useState(true)
  const [listError, setListError] = useState<string | null>(null)
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [summary, setSummary] = useState<TraceSummary | null>(null)
  const [analyzingFile, setAnalyzingFile] = useState<string | null>(null)
  const [summaryError, setSummaryError] = useState<string | null>(null)

  const refreshList = useCallback(async () => {
    setLoadingList(true)
    setListError(null)
    try {
      const list = await api.perfTraceList()
      setTraces([...list].sort((a, b) => b.modifiedAt - a.modifiedAt))
    } catch {
      setListError('Could not read the logs folder.')
    } finally {
      setLoadingList(false)
    }
  }, [api])

  // Initial recording state — onPerfTraceStateChanged only fires on change.
  useEffect(() => {
    let cancelled = false
    void Promise.all([api.perfTraceGetState(), api.perfPanZoomGetState()]).then(
      ([nextTraceState, nextPanZoomState]) => {
        if (cancelled) return
        setTraceState(nextTraceState)
        setPanZoomState(nextPanZoomState)
      },
    )
    return () => {
      cancelled = true
    }
  }, [api])

  useEffect(
    () =>
      api.onPerfTraceStateChanged((state) => {
        setTraceState((prev) => {
          if (prev.status === 'stopping' && state.status === 'idle') {
            // Recording just stopped (manually or via the 30s auto-stop) —
            // the new file won't show up until we ask again.
            void refreshList()
          }
          return state
        })
      }),
    [api, refreshList],
  )

  useEffect(
    () => api.onPerfPanZoomStateChanged(setPanZoomState),
    [api],
  )

  useEffect(() => {
    void refreshList()
  }, [refreshList])

  useEffect(() => {
    if (!traceState.recording || traceState.startedAt === null) {
      setElapsedSeconds(0)
      return
    }
    const startedAt = traceState.startedAt
    const tick = () =>
      setElapsedSeconds(
        Math.min(RECORD_MAX_SECONDS, Math.floor((Date.now() - startedAt) / 1000)),
      )
    tick()
    const id = setInterval(tick, 250)
    return () => clearInterval(id)
  }, [traceState.recording, traceState.startedAt])

  const handleAnalyze = async (fileName: string) => {
    setAnalyzingFile(fileName)
    setSummaryError(null)
    try {
      const result = await api.perfTraceGetSummary(fileName)
      if (result) {
        setSummary(result)
        setSelectedFile(fileName)
      } else {
        setSummaryError(`Could not parse ${fileName}.`)
      }
    } catch {
      setSummaryError(`Could not parse ${fileName}.`)
    } finally {
      setAnalyzingFile(null)
    }
  }

  const handleTraceToggle = async () => {
    setActionError(null)
    try {
      await api.perfTraceToggle()
    } catch (error) {
      setActionError(error instanceof Error ? error.message : 'Could not change trace state.')
    }
  }

  const handlePanZoomTest = async () => {
    setActionError(null)
    try {
      if (panZoomState.running) {
        await api.perfPanZoomStop()
        return
      }
      const result = await api.perfPanZoomRun()
      await refreshList()
      await handleAnalyze(result.fileName)
    } catch (error) {
      setActionError(error instanceof Error ? error.message : 'Could not run the pan/zoom test.')
    }
  }

  return (
    <div className="flex h-full w-full min-w-0 flex-col">
      <PerformanceHeader
        traceState={traceState}
        panZoomState={panZoomState}
        elapsedSeconds={elapsedSeconds}
        onToggle={() => void handleTraceToggle()}
        onPanZoomTest={() => void handlePanZoomTest()}
        onRefresh={() => void refreshList()}
      />
      {actionError ? (
        <div className="border-b border-[var(--surface-popover-border)] px-4 py-2 text-[11px] text-red-500">
          {actionError}
        </div>
      ) : null}
      <div className="min-h-0 flex-1 overflow-y-auto">
        <TraceList
          traces={traces}
          loading={loadingList}
          error={listError}
          selectedFile={selectedFile}
          analyzingFile={analyzingFile}
          onAnalyze={(fileName) => void handleAnalyze(fileName)}
          onReveal={(fileName) => api.perfTraceReveal(fileName)}
        />
        {summaryError ? (
          <div className="px-4 py-2 text-[11px] text-red-500">{summaryError}</div>
        ) : null}
        {summary ? <SummaryView summary={summary} fileName={selectedFile} /> : null}
      </div>
    </div>
  )
}

function PerformanceHeader({
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
  const traceBusy = traceState.status === 'starting' || traceState.status === 'stopping'
  const recording = traceState.status === 'recording'
  return (
    <div className="shrink-0 border-b border-[var(--surface-popover-border)] px-4 py-3">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[13px] font-semibold">Performance</div>
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
            disabled={traceBusy || panZoomState.running}
            className={
              recording || traceState.status === 'stopping'
                ? 'flex items-center gap-2 rounded border border-red-400/60 bg-red-500/10 px-2 py-1 text-[11px] font-medium text-red-600 hover:bg-red-500/20 disabled:opacity-50 dark:border-red-500/50 dark:text-red-400'
                : 'flex items-center gap-2 rounded border border-zinc-300 px-2 py-1 text-[11px] font-medium hover:bg-zinc-100 disabled:opacity-50 dark:border-zinc-700 dark:hover:bg-zinc-800'
            }
          >
            {traceState.status === 'stopping' ? (
              <span>Saving…</span>
            ) : traceState.status === 'starting' ? (
              <span>Starting…</span>
            ) : recording ? (
              <>
                <span className="h-1.5 w-1.5 shrink-0 animate-pulse rounded-full bg-red-500" />
                <span className="tabular-nums">{formatClock(elapsedSeconds)}</span>
                <span>Stop</span>
              </>
            ) : (
              <span>Record trace</span>
            )}
          </button>
        </div>
      </div>
      <div className="mt-1 text-[10px] leading-snug text-zinc-500 dark:text-zinc-400">
        Traces are saved to the app logs folder; recording auto-stops after{' '}
        {RECORD_MAX_SECONDS}s.
      </div>
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
          onClick={onPanZoomTest}
          disabled={!panZoomState.running && traceState.status !== 'idle'}
          className={
            panZoomState.running
              ? 'shrink-0 rounded border border-red-400/60 bg-red-500/10 px-2 py-1 text-[11px] font-medium text-red-600 hover:bg-red-500/20 dark:border-red-500/50 dark:text-red-400'
              : 'shrink-0 rounded border border-zinc-300 px-2 py-1 text-[11px] font-medium hover:bg-zinc-100 disabled:opacity-50 dark:border-zinc-700 dark:hover:bg-zinc-800'
          }
        >
          {panZoomState.running ? 'Stop test' : 'Run test'}
        </button>
      </div>
    </div>
  )
}

function TraceList({
  traces,
  loading,
  error,
  selectedFile,
  analyzingFile,
  onAnalyze,
  onReveal,
}: {
  traces: PerfTraceFileEntry[]
  loading: boolean
  error: string | null
  selectedFile: string | null
  analyzingFile: string | null
  onAnalyze: (fileName: string) => void
  onReveal: (fileName: string) => void
}) {
  if (loading) {
    return <div className="px-4 py-3 text-[11px] opacity-60">Loading traces…</div>
  }
  if (error) {
    return <div className="px-4 py-3 text-[11px] text-red-500">{error}</div>
  }
  if (traces.length === 0) {
    return (
      <div className="px-4 py-3 text-[11px] leading-snug text-zinc-500 dark:text-zinc-400">
        Record a trace, then analyze it here. Traces also open in ui.perfetto.dev.
      </div>
    )
  }

  return (
    <div className="flex flex-col divide-y divide-[var(--surface-popover-border)] border-b border-[var(--surface-popover-border)]">
      {traces.map((trace) => (
        <div
          key={trace.fileName}
          className={
            selectedFile === trace.fileName
              ? 'flex items-center gap-3 bg-zinc-100 px-4 py-2 dark:bg-zinc-800/60'
              : 'flex items-center gap-3 px-4 py-2'
          }
        >
          <div className="min-w-0 flex-1">
            <div className="truncate text-[12px]" title={trace.fileName}>
              {trace.fileName}
            </div>
            <div className="text-[10px] text-zinc-500 dark:text-zinc-400">
              {humanBytes(trace.sizeBytes)} · {formatModified(trace.modifiedAt)}
            </div>
          </div>
          <button
            type="button"
            onClick={() => onAnalyze(trace.fileName)}
            disabled={analyzingFile === trace.fileName}
            className="shrink-0 rounded border border-zinc-300 px-2 py-1 text-[11px] hover:bg-zinc-100 disabled:opacity-50 dark:border-zinc-700 dark:hover:bg-zinc-800"
          >
            {analyzingFile === trace.fileName ? 'Analyzing…' : 'Analyze'}
          </button>
          <button
            type="button"
            onClick={() => onReveal(trace.fileName)}
            className="shrink-0 rounded border border-zinc-300 px-2 py-1 text-[11px] hover:bg-zinc-100 dark:border-zinc-700 dark:hover:bg-zinc-800"
          >
            Reveal
          </button>
        </div>
      ))}
    </div>
  )
}

function SummaryView({
  summary,
  fileName,
}: {
  summary: TraceSummary
  fileName: string | null
}) {
  const threadItems = summary.threads.slice(0, MAX_BARS).map((t) => ({
    key: `${t.pid}:${t.tid}`,
    label: `${t.process} — ${t.thread}`,
    value: t.busyMs,
  }))
  const eventItems = summary.topEvents.slice(0, MAX_BARS).map((e) => ({
    key: e.name,
    label: `${e.name} (${e.count}×)`,
    value: e.totalMs,
  }))
  const timelineSeries = summary.timeline.map((s) => ({
    key: s.key,
    label: s.label,
    values: s.busyMs,
  }))

  return (
    <div className="perf-viz flex flex-col gap-6 px-4 py-4">
      {fileName ? (
        <div className="text-[11px] text-zinc-500 dark:text-zinc-400">Analyzed {fileName}</div>
      ) : null}

      <div className="flex flex-wrap items-baseline gap-x-6 gap-y-2">
        <Stat label="Duration" value={`${(summary.durationMs / 1000).toFixed(2)} s`} />
        <Stat label="Events" value={summary.eventCount.toLocaleString()} />
        {summary.markers.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            {summary.markers.map((marker) => (
              <span
                key={marker.label}
                className="rounded-full border border-zinc-300 px-2 py-0.5 text-[11px] text-zinc-600 dark:border-zinc-700 dark:text-zinc-300"
                title={marker.label}
              >
                {truncateLabel(marker.label)} — {marker.count} ({formatMs(marker.totalMs)})
              </span>
            ))}
          </div>
        ) : null}
      </div>

      <Section title="Busiest threads">
        {threadItems.length > 0 ? (
          <HorizontalBarChart items={threadItems} />
        ) : (
          <EmptyNote text="No thread activity recorded." />
        )}
      </Section>

      <Section title="Top events by total time">
        {eventItems.length > 0 ? (
          <HorizontalBarChart items={eventItems} />
        ) : (
          <EmptyNote text="No events recorded." />
        )}
      </Section>

      <Section title="Thread activity over time">
        {timelineSeries.length > 0 ? (
          <TimelineChart series={timelineSeries} bucketMs={summary.bucketMs} />
        ) : (
          <EmptyNote text="No timeline data recorded." />
        )}
      </Section>
    </div>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div>
      <div className="mb-2 text-[11px] font-semibold uppercase tracking-wider opacity-60">
        {title}
      </div>
      {children}
    </div>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[10px] uppercase tracking-wider opacity-60">{label}</div>
      <div className="text-[13px] tabular-nums">{value}</div>
    </div>
  )
}

function EmptyNote({ text }: { text: string }) {
  return <div className="text-[11px] opacity-55">{text}</div>
}

function truncateLabel(label: string): string {
  return label.length > MAX_MARKER_LABEL ? `${label.slice(0, MAX_MARKER_LABEL - 1)}…` : label
}
