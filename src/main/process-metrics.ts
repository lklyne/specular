/**
 * Samples `app.getAppMetrics()` and attributes each OS process to the views it
 * hosts, so the debug window can show where memory, CPU, and wakeups go.
 *
 * Deliberately read-only and renderer-free: it never calls `executeJavaScript`
 * or otherwise touches a page's main thread. Waking a throttled renderer to ask
 * how throttled it is would destroy the thing being measured — see
 * `visibility-probe.ts` for the one place that accepts that cost, on demand.
 */

import { app, webContents, type WebContentsView } from 'electron'
import type {
  ProcessMetricRow,
  ProcessMetricsSample,
  ViewOwner,
  ViewPresentation,
} from '../shared/process-metrics'
import { pages } from './runtime/runtime-context'
import { idleThrottleState } from './runtime/page-idle-throttle'
import { listComponentViews } from './runtime/component-page-factory'
import {
  aboveView,
  bgView,
  devtoolsBackgroundView,
  devtoolsHeaderView,
  devtoolsResizeHandleView,
  devtoolsView,
  leftSidebarView,
  toolbarView,
} from './runtime/view-refs'

/**
 * How a view is presented, from the state Chromium actually acts on.
 *
 * `getVisible()` is the throttling-relevant bit; zero-size bounds only stop
 * the view contributing to the composited frame.
 */
export function presentationOf(view: WebContentsView): ViewPresentation {
  if (typeof view.getVisible === 'function' && !view.getVisible()) return 'hidden'
  const bounds = view.getBounds()
  if (bounds.width === 0 || bounds.height === 0) return 'culled'
  return 'visible'
}

export function pageLabel(pageId: string): string {
  const page = pages.find((candidate) => candidate.id === pageId)
  if (!page) return pageId
  const named = page.name?.trim() || page.title?.trim()
  if (named) return named
  try {
    return new URL(page.url).host || page.url
  } catch {
    return page.url
  }
}

/** webContents.id → owner, for every view the runtime can name. */
function knownOwners(): Map<number, ViewOwner> {
  const owners = new Map<number, ViewOwner>()

  const addSingleton = (view: WebContentsView | null, label: string) => {
    if (!view || view.webContents.isDestroyed()) return
    owners.set(view.webContents.id, {
      label,
      kind: 'overlay',
      presentation: presentationOf(view),
    })
  }

  addSingleton(bgView, 'Canvas')
  addSingleton(aboveView, 'Above-view overlay')
  addSingleton(leftSidebarView, 'Left sidebar')
  addSingleton(toolbarView, 'Toolbar')
  addSingleton(devtoolsBackgroundView, 'DevTools background')
  addSingleton(devtoolsHeaderView, 'DevTools header')
  addSingleton(devtoolsResizeHandleView, 'DevTools resize handle')
  addSingleton(devtoolsView, 'DevTools')

  for (const page of pages) {
    const label = pageLabel(page.id)
    if (!page.pageView.webContents.isDestroyed()) {
      owners.set(page.pageView.webContents.id, {
        label,
        kind: 'page',
        pageId: page.id,
        url: page.url,
        presentation: presentationOf(page.pageView),
        cpuThrottleRate: page.lastCpuThrottleRate,
      })
    }
    if (page.devtoolsHostView && !page.devtoolsHostView.webContents.isDestroyed()) {
      owners.set(page.devtoolsHostView.webContents.id, {
        label: `${label} — devtools host`,
        kind: 'devtools',
        pageId: page.id,
        presentation: presentationOf(page.devtoolsHostView),
      })
    }
  }

  for (const cv of listComponentViews()) {
    if (cv.view.webContents.isDestroyed()) continue
    owners.set(cv.view.webContents.id, {
      label: `Component ${cv.entityId}`,
      kind: 'component',
      url: cv.loadedUrl ?? undefined,
      presentation: presentationOf(cv.view),
    })
  }

  return owners
}

/** Fallback label for a webContents the runtime does not track (debug window,
 *  settings, onboarding, a devtools front-end Chromium opened itself). */
function fallbackOwner(contents: Electron.WebContents): ViewOwner {
  const title = contents.getTitle().trim()
  if (title) return { label: title, kind: 'other' }
  const url = contents.getURL()
  if (!url) return { label: contents.getType(), kind: 'other' }
  try {
    const parsed = new URL(url)
    return { label: parsed.host || parsed.pathname, kind: 'other', url }
  } catch {
    return { label: url, kind: 'other', url }
  }
}

/** pid → the views it hosts. Many-to-one: same-site pages share a renderer. */
function ownersByPid(): Map<number, ViewOwner[]> {
  const known = knownOwners()
  const byPid = new Map<number, ViewOwner[]>()

  for (const contents of webContents.getAllWebContents()) {
    if (contents.isDestroyed()) continue
    let pid = 0
    try {
      pid = contents.getOSProcessId()
    } catch {
      continue
    }
    // A crashed or not-yet-spawned renderer reports 0 and matches no metric.
    if (!pid) continue
    const owner = known.get(contents.id) ?? fallbackOwner(contents)
    const list = byPid.get(pid)
    if (list) list.push(owner)
    else byPid.set(pid, [owner])
  }

  return byPid
}

export function sampleProcessMetrics(): ProcessMetricsSample {
  const byPid = ownersByPid()
  const rows: ProcessMetricRow[] = app.getAppMetrics().map((metric) => ({
    pid: metric.pid,
    type: metric.type,
    name: metric.name,
    workingSetKb: metric.memory.workingSetSize,
    peakWorkingSetKb: metric.memory.peakWorkingSetSize,
    cpuPercent: metric.cpu.percentCPUUsage,
    idleWakeupsPerSecond: metric.cpu.idleWakeupsPerSecond,
    cumulativeCpuSeconds: metric.cpu.cumulativeCPUUsage,
    owners: byPid.get(metric.pid) ?? (metric.pid === process.pid ? [{ label: 'Main', kind: 'other' }] : []),
  }))

  let pagesVisible = 0
  let pagesCulled = 0
  let pagesHidden = 0
  let pagesThrottled = 0
  for (const page of pages) {
    switch (presentationOf(page.pageView)) {
      case 'visible': pagesVisible += 1; break
      case 'culled': pagesCulled += 1; break
      case 'hidden': pagesHidden += 1; break
    }
    if ((page.lastCpuThrottleRate ?? 1) > 1) pagesThrottled += 1
  }

  return {
    sampledAt: Date.now(),
    rows,
    totals: {
      processCount: rows.length,
      workingSetKb: rows.reduce((sum, row) => sum + row.workingSetKb, 0),
      cpuPercent: rows.reduce((sum, row) => sum + row.cpuPercent, 0),
      idleWakeupsPerSecond: rows.reduce((sum, row) => sum + row.idleWakeupsPerSecond, 0),
      pagesVisible,
      pagesCulled,
      pagesHidden,
      pagesThrottled,
    },
    idleThrottle: idleThrottleState(),
  }
}
