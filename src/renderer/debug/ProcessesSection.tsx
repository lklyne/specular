/**
 * Processes debug panel — live per-process memory, CPU, and wakeups, attributed
 * to the canvas pages and overlays each process hosts, plus the throttling
 * probe that explains what the culled pages are doing.
 */

import { useState } from 'react'
import type {
  ProcessMetricsSample,
  VisibilityProbeResult,
} from '../../shared/process-metrics'
import type { DebugElectronAPI } from '../../shared/electron-api/debug'
import { humanBytes } from './format'
import { formatMetricsReport } from './metrics-report'
import { ProcessTable } from './ProcessTable'
import { useProcessMetrics } from './useProcessMetrics'
import { VisibilityProbePanel } from './VisibilityProbePanel'

const DEFAULT_INTERVAL_MS = 2000

const INTERVALS: { label: string; ms: number | null }[] = [
  { label: '1s', ms: 1000 },
  { label: '2s', ms: 2000 },
  { label: '5s', ms: 5000 },
  { label: 'Hold', ms: null },
]

export function ProcessesSection({ api }: { api: DebugElectronAPI }) {
  const [intervalMs, setIntervalMs] = useState<number | null>(DEFAULT_INTERVAL_MS)
  const [probeResult, setProbeResult] = useState<VisibilityProbeResult | null>(null)
  const [copied, setCopied] = useState(false)
  const { sample, error, refresh } = useProcessMetrics(api, intervalMs)

  const copyReport = () => {
    api.copyText(formatMetricsReport(sample, probeResult))
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <div className="flex h-full w-full min-w-0 flex-col">
      <div className="shrink-0 border-b border-[var(--surface-popover-border)] px-4 py-3">
        <div className="flex items-center justify-between gap-3">
          <div className="text-[13px] font-semibold">Processes</div>
          <div className="flex items-center gap-1">
            {INTERVALS.map((choice) => (
              <button
                key={choice.label}
                type="button"
                onClick={() => setIntervalMs(choice.ms)}
                className={
                  intervalMs === choice.ms
                    ? 'rounded border border-zinc-400 bg-zinc-200 px-2 py-1 text-[11px] dark:border-zinc-600 dark:bg-zinc-800'
                    : 'rounded border border-zinc-300 px-2 py-1 text-[11px] hover:bg-zinc-100 dark:border-zinc-700 dark:hover:bg-zinc-800'
                }
              >
                {choice.label}
              </button>
            ))}
            <button
              type="button"
              onClick={() => void refresh()}
              className="rounded border border-zinc-300 px-2 py-1 text-[11px] hover:bg-zinc-100 dark:border-zinc-700 dark:hover:bg-zinc-800"
            >
              Refresh
            </button>
            <button
              type="button"
              onClick={copyReport}
              disabled={!sample && !probeResult}
              title="Copy the table and any probe result as Markdown"
              className="rounded border border-zinc-300 px-2 py-1 text-[11px] hover:bg-zinc-100 disabled:opacity-50 dark:border-zinc-700 dark:hover:bg-zinc-800"
            >
              {copied ? 'Copied' : 'Copy'}
            </button>
          </div>
        </div>
        {sample ? <Totals sample={sample} /> : null}
      </div>

      {error ? <div className="px-4 py-2 text-[11px] text-red-500">{error}</div> : null}

      <div className="min-h-0 flex-1 overflow-y-auto">
        {sample ? (
          <ProcessTable rows={sample.rows} />
        ) : (
          <div className="px-4 py-3 text-[11px] opacity-60">Sampling…</div>
        )}
        <VisibilityProbePanel
          api={api}
          result={probeResult}
          onResult={setProbeResult}
        />
      </div>
    </div>
  )
}

function Totals({ sample }: { sample: ProcessMetricsSample }) {
  const { totals } = sample
  return (
    <div className="mt-2 flex flex-wrap items-baseline gap-x-5 gap-y-1">
      <Stat label="Processes" value={String(totals.processCount)} />
      <Stat label="Memory" value={humanBytes(totals.workingSetKb * 1024)} />
      <Stat label="CPU" value={`${totals.cpuPercent.toFixed(1)}%`} />
      <Stat label="Wakeups/s" value={totals.idleWakeupsPerSecond.toFixed(0)} />
      <Stat
        label="Pages"
        value={`${totals.pagesVisible} visible · ${totals.pagesCulled} culled · ${totals.pagesHidden} hidden`}
      />
      <Stat label="Idle throttle" value={describeIdleThrottle(sample)} />
    </div>
  )
}

/**
 * The A/B arm this sample belongs to. Stated plainly because a sample taken
 * while the debug window holds focus is never throttled, and a reader
 * comparing two numbers needs to know which is which.
 */
function describeIdleThrottle(sample: ProcessMetricsSample): string {
  const { idleThrottle, totals } = sample
  if (!idleThrottle.idle) {
    if (idleThrottle.windowFocused) return 'awake — focused'
    if (idleThrottle.awakeHoldCount > 0) return `awake — ${idleThrottle.awakeHoldCount} hold(s)`
    return 'awake — grace'
  }
  return `idle · ${totals.pagesFrozen} frozen`
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[10px] uppercase tracking-wider opacity-60">{label}</div>
      <div className="text-[12px] tabular-nums">{value}</div>
    </div>
  )
}
