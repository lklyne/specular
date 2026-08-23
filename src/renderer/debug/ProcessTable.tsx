/**
 * Sortable per-process table. One row per OS process, with the presentation of
 * each view it hosts tagged inline — Chromium coalesces same-site pages into
 * one renderer, so a row's memory is only attributable to a single page when
 * it owns one.
 */

import { useMemo, useState } from 'react'
import type { ProcessMetricRow, ViewOwner, ViewPresentation } from '../../shared/process-metrics'
import { humanBytes } from './format'

type SortKey = 'label' | 'type' | 'pid' | 'memory' | 'cpu' | 'wakeups' | 'cpuTotal'

const COLUMNS: { key: SortKey; label: string; numeric: boolean }[] = [
  { key: 'label', label: 'Process', numeric: false },
  { key: 'type', label: 'Type', numeric: false },
  { key: 'pid', label: 'PID', numeric: true },
  { key: 'memory', label: 'Memory', numeric: true },
  { key: 'cpu', label: 'CPU', numeric: true },
  { key: 'wakeups', label: 'Wakeups/s', numeric: true },
  { key: 'cpuTotal', label: 'CPU total', numeric: true },
]

const PRESENTATION_CLASS: Record<ViewPresentation, string> = {
  visible: 'border-emerald-400/50 text-emerald-600 dark:text-emerald-400',
  // Amber, not green: culled means off-screen but still fully awake.
  culled: 'border-amber-400/60 text-amber-600 dark:text-amber-400',
  hidden: 'border-zinc-300 text-zinc-500 dark:border-zinc-700 dark:text-zinc-400',
}

function primaryLabel(row: ProcessMetricRow): string {
  if (row.owners.length === 0) return row.name ?? row.type
  if (row.owners.length === 1) return row.owners[0].label
  return `${row.owners[0].label} +${row.owners.length - 1}`
}

function sortValue(row: ProcessMetricRow, key: SortKey): number | string {
  switch (key) {
    case 'label': return primaryLabel(row).toLowerCase()
    case 'type': return row.type
    case 'pid': return row.pid
    case 'memory': return row.workingSetKb
    case 'cpu': return row.cpuPercent
    case 'wakeups': return row.idleWakeupsPerSecond
    case 'cpuTotal': return row.cumulativeCpuSeconds ?? 0
  }
}

export function ProcessTable({ rows }: { rows: ProcessMetricRow[] }) {
  const [sortKey, setSortKey] = useState<SortKey>('memory')
  const [ascending, setAscending] = useState(false)

  const sorted = useMemo(() => {
    const copy = [...rows]
    copy.sort((a, b) => {
      const left = sortValue(a, sortKey)
      const right = sortValue(b, sortKey)
      const order =
        typeof left === 'string' && typeof right === 'string'
          ? left.localeCompare(right)
          : Number(left) - Number(right)
      return ascending ? order : -order
    })
    return copy
  }, [rows, sortKey, ascending])

  const toggleSort = (key: SortKey) => {
    if (key === sortKey) {
      setAscending((previous) => !previous)
      return
    }
    setSortKey(key)
    // Names read best A→Z; every measurement reads best worst-first.
    setAscending(key === 'label' || key === 'type')
  }

  return (
    <table className="w-full border-collapse text-[11px]">
      <thead className="sticky top-0 bg-[var(--surface-popover,#fff)] dark:bg-zinc-900">
        <tr className="border-b border-[var(--surface-popover-border)]">
          {COLUMNS.map((column) => (
            <th
              key={column.key}
              className={`whitespace-nowrap px-3 py-1.5 font-medium uppercase tracking-wider ${
                column.numeric ? 'text-right' : 'text-left'
              }`}
            >
              <button
                type="button"
                onClick={() => toggleSort(column.key)}
                className="whitespace-nowrap text-[10px] uppercase tracking-wider"
              >
                {column.label}
                {sortKey === column.key ? (ascending ? '\u00a0↑' : '\u00a0↓') : ''}
              </button>
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {sorted.map((row) => (
          <tr
            key={row.pid}
            className="border-b border-[var(--surface-popover-border)] whitespace-nowrap"
          >
            <td className="w-full max-w-0 px-3 py-1.5">
              <div className="flex min-w-0 items-center gap-1.5">
                <span className="truncate" title={primaryLabel(row)}>
                  {primaryLabel(row)}
                </span>
                {row.owners.length > 0 ? <OwnerTags owners={row.owners} /> : null}
              </div>
            </td>
            <td className="px-3 py-1.5 opacity-70">
              {row.type}
              {row.name && row.name !== row.type ? ` · ${row.name}` : ''}
            </td>
            <td className="px-3 py-1.5 text-right tabular-nums opacity-70">{row.pid}</td>
            <td className="px-3 py-1.5 text-right tabular-nums">
              {humanBytes(row.workingSetKb * 1024)}
            </td>
            <td className="px-3 py-1.5 text-right tabular-nums">
              {row.cpuPercent.toFixed(1)}%
            </td>
            <td className="px-3 py-1.5 text-right tabular-nums">
              {row.idleWakeupsPerSecond.toFixed(0)}
            </td>
            <td className="px-3 py-1.5 text-right tabular-nums opacity-70">
              {row.cumulativeCpuSeconds === undefined
                ? '—'
                : `${row.cumulativeCpuSeconds.toFixed(1)}s`}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

function OwnerTags({ owners }: { owners: ViewOwner[] }) {
  return (
    <>
      {owners.map((owner, index) =>
        owner.presentation ? (
          <span
            key={`${owner.label}-${index}`}
            className={`shrink-0 rounded-full border px-1.5 py-px text-[10px] ${PRESENTATION_CLASS[owner.presentation]}`}
            title={owner.url ?? owner.label}
          >
            {owner.presentation}
          </span>
        ) : null,
      )}
    </>
  )
}
