/**
 * Runs and renders the culled-page throttling probe.
 *
 * Reads as a verdict, not a data dump: the question is whether
 * `View.setVisible(false)` gets Chromium to treat a culled page as hidden, and
 * the answer is "frames went to zero and timers fell to ~1/s" or it isn't.
 */

import { useState } from 'react'
import type { DebugElectronAPI } from '../../shared/electron-api/debug'
import type { VisibilityProbeResult, VisibilityProbeSample } from '../../shared/process-metrics'
import { perSecond, VERDICT_LABEL, verdictOf, type Verdict } from './probe-verdict'

export function VisibilityProbePanel({
  api,
  result,
  onResult,
}: {
  api: DebugElectronAPI
  result: VisibilityProbeResult | null
  onResult: (result: VisibilityProbeResult) => void
}) {
  const [running, setRunning] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const run = async () => {
    setRunning(true)
    setError(null)
    try {
      onResult(await api.visibilityProbeRun())
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : 'Probe failed.')
    } finally {
      setRunning(false)
    }
  }

  return (
    <div className="border-t border-[var(--surface-popover-border)] px-4 py-3">
      <div className="flex items-center justify-between gap-3">
        <div>
          <div className="text-[12px] font-semibold">Throttling probe</div>
          <div className="mt-0.5 max-w-prose text-[10px] leading-snug text-zinc-500 dark:text-zinc-400">
            Measures every culled page as-is, flips <code>setVisible(false)</code>, measures
            again, restores. Pan pages off-screen first — culled pages are the only ones probed.
          </div>
        </div>
        <button
          type="button"
          onClick={() => void run()}
          disabled={running}
          className="shrink-0 rounded border border-zinc-300 px-2 py-1 text-[11px] font-medium hover:bg-zinc-100 disabled:opacity-50 dark:border-zinc-700 dark:hover:bg-zinc-800"
        >
          {running ? 'Probing…' : 'Run probe'}
        </button>
      </div>

      {error ? <div className="mt-2 text-[11px] text-red-500">{error}</div> : null}
      {result?.note ? (
        <div className="mt-2 text-[11px] text-zinc-500 dark:text-zinc-400">{result.note}</div>
      ) : null}

      {result && result.pages.length > 0 ? (
        <table className="mt-3 w-full border-collapse text-[11px]">
          <thead>
            <tr className="border-b border-[var(--surface-popover-border)] text-[10px] uppercase tracking-wider opacity-60">
              <th className="px-2 py-1 text-left font-medium">Page</th>
              <th className="px-2 py-1 text-left font-medium">Culled</th>
              <th className="px-2 py-1 text-left font-medium">setVisible(false)</th>
              <th className="px-2 py-1 text-right font-medium">Verdict</th>
            </tr>
          </thead>
          <tbody>
            {result.pages.map((page) => (
              <tr
                key={page.pageId}
                className="border-b border-[var(--surface-popover-border)]"
              >
                <td className="px-2 py-1.5">
                  <div className="truncate" title={page.url}>{page.label}</div>
                  {page.error ? (
                    <div className="text-[10px] text-red-500">{page.error}</div>
                  ) : null}
                </td>
                <td className="px-2 py-1.5"><SampleCell sample={page.before} /></td>
                <td className="px-2 py-1.5"><SampleCell sample={page.after} /></td>
                <td className="px-2 py-1.5 text-right">
                  <VerdictTag verdict={verdictOf(page)} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </div>
  )
}

function SampleCell({ sample }: { sample: VisibilityProbeSample | null }) {
  if (!sample) return <span className="opacity-50">—</span>
  return (
    <div className="tabular-nums">
      <div>{sample.visibilityState}</div>
      <div className="text-[10px] opacity-70">
        {perSecond(sample.frames, sample.elapsedMs).toFixed(0)} fps ·{' '}
        {perSecond(sample.timerTicks, sample.elapsedMs).toFixed(1)} timers/s
      </div>
    </div>
  )
}

function VerdictTag({ verdict }: { verdict: Verdict }) {
  const style =
    verdict === 'throttled'
      ? 'border-emerald-400/50 text-emerald-600 dark:text-emerald-400'
      : verdict === 'unchanged'
        ? 'border-red-400/50 text-red-600 dark:text-red-400'
        : 'border-zinc-300 opacity-60 dark:border-zinc-700'
  return (
    <span className={`rounded-full border px-1.5 py-px text-[10px] ${style}`}>
      {VERDICT_LABEL[verdict]}
    </span>
  )
}
