/**
 * Renders a metrics sample and probe result as Markdown, for pasting into an
 * issue or a chat with an agent. Pure — no React, no clipboard.
 */

import type {
  ProcessMetricRow,
  ProcessMetricsSample,
  VisibilityProbeResult,
} from '../../shared/process-metrics'
import { humanBytes } from './format'
import { describeSample, VERDICT_LABEL, verdictOf } from './probe-verdict'

/** Owners beyond this are summarized, so a shared renderer row stays readable. */
const MAX_LISTED_OWNERS = 4

function ownerSummary(row: ProcessMetricRow): string {
  if (row.owners.length === 0) return row.name ?? row.type
  const listed = row.owners.slice(0, MAX_LISTED_OWNERS).map((owner) => {
    const notes = [owner.presentation, throttleNote(owner)].filter(Boolean)
    return notes.length > 0 ? `${owner.label} (${notes.join(', ')})` : owner.label
  })
  const remaining = row.owners.length - listed.length
  return remaining > 0 ? `${listed.join('; ')} +${remaining} more` : listed.join('; ')
}

/** Only worth printing when a page is actually slowed. */
function throttleNote(owner: { cpuThrottleRate?: number }): string | null {
  const rate = owner.cpuThrottleRate
  return rate !== undefined && rate > 1 ? `throttled ${rate}x` : null
}

function markdownTable(header: string[], rows: string[][]): string {
  const lines = [
    `| ${header.join(' | ')} |`,
    `| ${header.map(() => '---').join(' | ')} |`,
    ...rows.map((row) => `| ${row.join(' | ')} |`),
  ]
  return lines.join('\n')
}

function processSection(sample: ProcessMetricsSample): string {
  const { totals } = sample
  const headline = [
    `Processes ${totals.processCount}`,
    `Memory ${humanBytes(totals.workingSetKb * 1024)}`,
    `CPU ${totals.cpuPercent.toFixed(1)}%`,
    `Wakeups/s ${totals.idleWakeupsPerSecond.toFixed(0)}`,
  ].join(' · ')
  const pageLine = `Pages: ${totals.pagesVisible} visible · ${totals.pagesCulled} culled · ${totals.pagesHidden} hidden`
  const { idleThrottle } = sample
  const throttleLine = `Idle throttle: ${
    idleThrottle.disabled
      ? 'disabled (SPECULAR_DISABLE_IDLE_THROTTLE=1)'
      : `${idleThrottle.idle ? 'idle' : 'awake'} · focused ${idleThrottle.windowFocused} · holds ${idleThrottle.awakeHoldCount} · ${totals.pagesThrottled} pages throttled`
  }`

  const rows = [...sample.rows]
    .sort((a, b) => b.workingSetKb - a.workingSetKb)
    .map((row) => [
      ownerSummary(row),
      row.name && row.name !== row.type ? `${row.type} · ${row.name}` : row.type,
      String(row.pid),
      humanBytes(row.workingSetKb * 1024),
      `${row.cpuPercent.toFixed(1)}%`,
      row.idleWakeupsPerSecond.toFixed(0),
      row.cumulativeCpuSeconds === undefined ? '—' : `${row.cumulativeCpuSeconds.toFixed(1)}s`,
    ])

  return [
    headline,
    pageLine,
    throttleLine,
    '',
    markdownTable(
      ['Process', 'Type', 'PID', 'Memory', 'CPU', 'Wakeups/s', 'CPU total'],
      rows,
    ),
  ].join('\n')
}

function probeSection(probe: VisibilityProbeResult): string {
  const parts = [`### Throttling probe (${probe.windowMs}ms windows)`]
  if (probe.note) parts.push('', probe.note)
  if (probe.pages.length > 0) {
    const rows = probe.pages.map((page) => [
      page.error ? `${page.label} — ${page.error}` : page.label,
      describeSample(page.before),
      describeSample(page.after),
      VERDICT_LABEL[verdictOf(page)],
    ])
    parts.push(
      '',
      markdownTable(['Page', 'Culled', 'setVisible(false)', 'Verdict'], rows),
    )
  }
  return parts.join('\n')
}

export function formatMetricsReport(
  sample: ProcessMetricsSample | null,
  probe: VisibilityProbeResult | null,
): string {
  const sections: string[] = []
  if (sample) {
    sections.push(
      `## Specular process metrics — ${new Date(sample.sampledAt).toLocaleString()}`,
      '',
      processSection(sample),
    )
  }
  if (probe) {
    if (sections.length > 0) sections.push('')
    sections.push(probeSection(probe))
  }
  if (sections.length === 0) return 'No metrics sampled yet.'
  return `${sections.join('\n')}\n`
}
