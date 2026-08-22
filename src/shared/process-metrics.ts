/**
 * Per-process metrics sampled from `app.getAppMetrics()`, with each OS process
 * attributed to the views it hosts.
 *
 * Attribution is many-to-one on purpose: Chromium coalesces same-site pages
 * into a single renderer process, so one row can own several canvas pages.
 * Reading a row's memory as "this page costs X" is only valid when `owners`
 * has one entry.
 */

/** How a view is presented right now — the battery-relevant axis. */
export type ViewPresentation =
  /** Painting and composited into the window. */
  | 'visible'
  /** Off-screen: bounds collapsed to 0×0, but still an attached, *visible*
   *  child — Chromium keeps rAF, timers, and media running at full rate. */
  | 'culled'
  /** `View.setVisible(false)` — detached from the compositor, and the state
   *  Chromium needs before background throttling can engage. */
  | 'hidden'

export type ViewOwnerKind = 'page' | 'component' | 'overlay' | 'devtools' | 'other'

export interface ViewOwner {
  /** Human label for the table, e.g. a page title or 'Canvas'. */
  label: string
  kind: ViewOwnerKind
  /** Present for canvas pages, so rows can be cross-referenced with a probe. */
  pageId?: string
  url?: string
  presentation?: ViewPresentation
}

export interface ProcessMetricRow {
  pid: number
  /** Electron's process type: 'Browser' | 'Tab' | 'GPU' | 'Utility' | … */
  type: string
  /** Service name for utility processes, e.g. 'Network Service'. */
  name?: string
  /** Resident memory in KB, as Chromium reports it. */
  workingSetKb: number
  peakWorkingSetKb: number
  /** Share of one core since the previous sample. Can exceed 100. */
  cpuPercent: number
  /** Idle wakeups per second — the closest in-process proxy for battery
   *  drain on Apple Silicon, where wakeups matter more than raw CPU. */
  idleWakeupsPerSecond: number
  /** Total CPU-seconds since this process started. Undefined on platforms
   *  that do not report it. */
  cumulativeCpuSeconds?: number
  owners: ViewOwner[]
}

export interface ProcessMetricsSample {
  /** Epoch ms. */
  sampledAt: number
  rows: ProcessMetricRow[]
  totals: {
    processCount: number
    workingSetKb: number
    cpuPercent: number
    idleWakeupsPerSecond: number
    /** Canvas pages by presentation — the counts the lifecycle work moves. */
    pagesVisible: number
    pagesCulled: number
    pagesHidden: number
  }
}

/** One measurement window against a single page. */
export interface VisibilityProbeSample {
  /** `document.visibilityState` as the page itself reports it. */
  visibilityState: string
  /** requestAnimationFrame callbacks during the window. */
  frames: number
  /** Ticks of a 100ms setInterval during the window. Chromium clamps
   *  background timers to ~1/s, so this separates "hidden" from "throttled". */
  timerTicks: number
  elapsedMs: number
}

export interface VisibilityProbePageResult {
  pageId: string
  label: string
  url: string
  /** State before the probe touched anything. */
  presentation: ViewPresentation
  /** Measured as-is, with the page in its current culled state. */
  before: VisibilityProbeSample | null
  /** Measured after `View.setVisible(false)`. */
  after: VisibilityProbeSample | null
  error?: string
}

export interface VisibilityProbeResult {
  probedAt: number
  /** Length of each of the two measurement windows. */
  windowMs: number
  pages: VisibilityProbePageResult[]
  /** Set when the probe had nothing to measure. */
  note?: string
}
