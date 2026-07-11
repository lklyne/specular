// fallow-ignore-file circular-dependencies
// Suppressed: see #141. document-commands → … → canvas-layout-data / layout-engine import presence-cursor back
import type { IncomingMessage } from 'http'
import type {
  PresenceActivity,
  PresenceLabelKey,
  PresenceSurface,
  PresenceTargetRect,
  PresenceTargetRefSource,
} from '../shared/types'
import {
  PRESENCE_STEP_DELAY_MS,
  PRESENCE_THINKING_DELAY_MS,
  selectDwellBudgetMs,
} from '../shared/presence-timing'
import { PRESENCE_LABEL_KEYS } from '../shared/presence-label-keys'
import {
  getTextEntities,
  getFileEntities,
} from './runtime/document-commands'
import { pages } from './runtime/runtime-context'
import { workspaceGroups } from './runtime/workspace-model'
import {
  resolveSession,
  mcpSessions,
  MCP_SESSION_TIMEOUT_MS,
} from './presence-session'

// --- Types ---

export interface PresenceCursorEntry {
  sessionId: string
  clientName: string
  color: string
  canvasX: number
  canvasY: number
  surface: PresenceSurface
  activity: PresenceActivity
  pageId?: string | null
  pageX?: number | null
  pageY?: number | null
  labelKey: PresenceLabelKey | null
  taskLabel?: string | null
  labelHint?: string | null
  labelParams?: Record<string, string | number | boolean> | null
  targetRef?: string | null
  targetRefSource?: PresenceTargetRefSource | null
  targetName?: string | null
  targetRect?: PresenceTargetRect | null
  updatedAt: number
  // Wall-clock time of the most recent canvasX/canvasY change. Drives the
  // CDP-proxy pre-click sleep so the budget resets when the cursor is
  // actually repositioned (not just re-tagged).
  lastMoveAt: number
  // Wall-clock time of the session's last real CDP dispatch (mouse press,
  // scroll) — the burst/gap signal `selectDwellBudgetMs` regimes on (ADR
  // 0029). Set by `recordPresenceAct`, independent of position changes.
  lastActAt: number | null
  // The dwell budget (ms) selected for the act currently in flight, stashed
  // on the cursor when it repositions so the CDP-proxy wait and the
  // broadcast the renderer reads from agree on the same number (ADR 0029:
  // "the renderer must finish travel within the dwell").
  dwellBudgetMs: number | null
  // The most recent labelKey representing a real agent intent (i.e. not
  // the synthetic 'thinking'/'idle'/'departing' states `scheduleThinkingState`
  // and `setPresenceCursorIdle` write). Those two overwrite `labelKey` so
  // the visible label reads "Thinking…"/nothing, which would otherwise
  // erase what the agent was actually doing right before the gap started.
  // Preserved across that overwrite (see `withLastIntentLabelKey`) so
  // `selectAmbientMode` (issue #319 Phase 3, presence-ambient.ts) can tell
  // a reading gap from a generic thinking gap.
  lastIntentLabelKey: PresenceLabelKey | null
}

export interface ActivePresenceTask {
  sessionId: string
  clientName: string
  taskLabel: string | null
  surface: PresenceSurface
  pageId: string | null
  pageX: number | null
  pageY: number | null
  canvasX: number | null
  canvasY: number | null
  targetName: string | null
  targetRect: PresenceTargetRect | null
  labelHint: string | null
  updatedAt: number
}

// --- State ---

export const presenceCursors = new Map<string, PresenceCursorEntry>()
export const activePresenceTasks = new Map<string, ActivePresenceTask>()
const presenceChangeListeners = new Set<() => void>()
let presenceExpiryTimer: NodeJS.Timeout | null = null

// --- Constants ---

export const PRESENCE_CURSOR_STEP_DELAY_MS = PRESENCE_STEP_DELAY_MS
/** Canvas-space distance below which a CDP-time reposition is treated as a
 *  no-op correction — the cursor stays where the intent placed it rather than
 *  restarting its animation to a few-pixel-off coordinate. */
export const PRESENCE_CURSOR_POSITION_SKIP_PX = 30
const PRESENCE_CURSOR_THINKING_DELAY_MS = PRESENCE_THINKING_DELAY_MS
const PRESENCE_DEPARTURE_GRACE_MS = 1500
const PRESENCE_IDLE_RETIRE_MS = 10_000

// --- Coercion validation sets ---

// 'departing' is set internally when a session winds down; it is never
// accepted from client input.
const COERCIBLE_PRESENCE_LABEL_KEYS = new Set<PresenceLabelKey>(
  PRESENCE_LABEL_KEYS.filter((key) => key !== 'departing'),
)

const PRESENCE_ACTIVITIES = new Set<PresenceActivity>([
  'traveling',
  'acting',
  'waiting',
  'thinking',
  'idle',
  'departing',
])

const PRESENCE_SURFACES = new Set<PresenceSurface>(['canvas', 'page'])

// The synthetic labelKeys `scheduleThinkingState`/`setPresenceCursorIdle`
// write over a cursor's real labelKey. Excluded from
// `lastIntentLabelKey` tracking — see that field's doc comment.
const SYNTHETIC_PRESENCE_LABEL_KEYS = new Set<PresenceLabelKey>(['thinking', 'idle', 'departing'])

/** Next value for `lastIntentLabelKey`: adopt `labelKey` if it's a real
 *  intent, otherwise carry forward whatever the cursor already had. */
function withLastIntentLabelKey(
  existing: PresenceCursorEntry | undefined,
  labelKey: PresenceLabelKey | null | undefined,
): PresenceLabelKey | null {
  if (labelKey && !SYNTHETIC_PRESENCE_LABEL_KEYS.has(labelKey)) return labelKey
  return existing?.lastIntentLabelKey ?? null
}

let thinkingTimer: NodeJS.Timeout | null = null
export let activeScanId = 0
export function bumpActiveScanId(): number {
  return ++activeScanId
}

// --- Presence core ---

export function notifyPresenceChanged(): void {
  for (const listener of presenceChangeListeners) listener()
}

function schedulePresenceExpiry(): void {
  if (presenceExpiryTimer) return
  presenceExpiryTimer = setTimeout(() => {
    presenceExpiryTimer = null
    const before = presenceCursors.size + activePresenceTasks.size
    expirePresenceCursors(Date.now())
    const after = presenceCursors.size + activePresenceTasks.size
    if (after !== before) notifyPresenceChanged()
    if (presenceCursors.size > 0 || activePresenceTasks.size > 0) {
      schedulePresenceExpiry()
    }
  }, 2_000)
}

export function deriveColor(sessionId: string): string {
  let hash = 0
  for (let i = 0; i < sessionId.length; i++) {
    hash = ((hash << 5) - hash + sessionId.charCodeAt(i)) | 0
  }
  const hue = ((hash % 360) + 360) % 360
  return `hsl(${hue}, 70%, 55%)`
}

export function removePresenceCursor(id: string): void {
  presenceCursors.delete(id)
}

/**
 * Transition a presence cursor to `departing` and schedule its removal.
 * Called from session close, CDP transport drop, and the expiry sweep when
 * the underlying MCP session has gone away. Safe to call multiple times
 * and when no cursor exists.
 */
export function beginPresenceDeparture(
  sessionId: string,
  removeAfterMs: number = PRESENCE_DEPARTURE_GRACE_MS,
): void {
  const hadTask = activePresenceTasks.delete(sessionId)
  const existing = presenceCursors.get(sessionId)
  if (!existing) {
    if (hadTask) notifyPresenceChanged()
    return
  }
  if (existing.activity === 'departing') return
  presenceCursors.set(sessionId, {
    ...existing,
    activity: 'departing',
    labelKey: null,
    updatedAt: Date.now(),
  })
  notifyPresenceChanged()
  setTimeout(() => {
    const current = presenceCursors.get(sessionId)
    if (!current || current.activity !== 'departing') return
    removePresenceCursor(sessionId)
    notifyPresenceChanged()
  }, removeAfterMs)
}

function isSessionLive(sessionId: string, now: number): boolean {
  const session = mcpSessions.get(sessionId)
  if (!session) return false
  return now - session.lastSeenAt <= MCP_SESSION_TIMEOUT_MS
}

function expirePresenceCursors(now: number): void {
  for (const [id, cursor] of presenceCursors) {
    if (cursor.activity === 'departing') {
      if (now - cursor.updatedAt > PRESENCE_DEPARTURE_GRACE_MS) {
        removePresenceCursor(id)
        activePresenceTasks.delete(id)
      }
      continue
    }
    if (!isSessionLive(id, now)) {
      beginPresenceDeparture(id)
      continue
    }
    if (now - cursor.updatedAt > PRESENCE_IDLE_RETIRE_MS) {
      beginPresenceDeparture(id)
    }
  }
  // Clean up orphaned active tasks whose cursors have already been removed.
  for (const id of activePresenceTasks.keys()) {
    if (!presenceCursors.has(id) && !isSessionLive(id, now)) {
      activePresenceTasks.delete(id)
    }
  }
}

export function getPresenceCursors(): PresenceCursorEntry[] {
  expirePresenceCursors(Date.now())
  return [...presenceCursors.values()]
}

export function onPresenceCursorsChanged(listener: () => void): () => void {
  presenceChangeListeners.add(listener)
  return () => { presenceChangeListeners.delete(listener) }
}

// --- Coercion helpers ---

export function coercePresenceLabelKey(value: unknown): PresenceLabelKey | null {
  return typeof value === 'string' && COERCIBLE_PRESENCE_LABEL_KEYS.has(value as PresenceLabelKey)
    ? (value as PresenceLabelKey)
    : null
}

export function coercePresenceActivity(value: unknown): PresenceActivity | null {
  return typeof value === 'string' && PRESENCE_ACTIVITIES.has(value as PresenceActivity)
    ? (value as PresenceActivity)
    : null
}

export function coercePresenceSurface(value: unknown): PresenceSurface | null {
  return typeof value === 'string' && PRESENCE_SURFACES.has(value as PresenceSurface)
    ? (value as PresenceSurface)
    : null
}

export function coercePresenceTargetRefSource(value: unknown): PresenceTargetRefSource | null {
  return value === 'specular' || value === 'agent-browser'
    ? value
    : null
}

// --- Cursor mutation functions ---

/** Copy `keys` from `source`, skipping keys whose value is `undefined` (so
 *  they fall through to the previous spread), but keeping explicit `null`s
 *  (which overwrite). */
function pickDefined<T extends object, K extends keyof T>(
  source: T | undefined,
  keys: readonly K[],
): Partial<Pick<T, K>> {
  const picked: Partial<Pick<T, K>> = {}
  if (!source) return picked
  for (const key of keys) {
    if (source[key] !== undefined) picked[key] = source[key]
  }
  return picked
}

/** Cursor fields merged patch-over-existing-over-null. The rest of
 *  `PresenceCursorEntry` has bespoke fallbacks and is set explicitly. */
const MERGED_CURSOR_FIELDS = [
  'pageId',
  'pageX',
  'pageY',
  'labelKey',
  'labelParams',
  'targetRef',
  'targetRefSource',
  'targetName',
  'targetRect',
  'dwellBudgetMs',
] as const

/** Task fields merged patch-over-existing-over-null. */
const MERGED_TASK_FIELDS = [
  'taskLabel',
  'pageId',
  'pageX',
  'pageY',
  'canvasX',
  'canvasY',
  'targetName',
  'targetRect',
  'labelHint',
] as const

export function upsertPresenceCursor(
  request: IncomingMessage,
  patch: {
    body?: Record<string, unknown>
    canvasX?: number
    canvasY?: number
    surface?: PresenceSurface
    activity?: PresenceActivity
    pageId?: string | null
    pageX?: number | null
    pageY?: number | null
    labelKey?: PresenceLabelKey | null
    taskLabel?: string | null
    labelHint?: string | null
    labelParams?: Record<string, string | number | boolean> | null
    targetRef?: string | null
    targetRefSource?: PresenceTargetRefSource | null
    targetName?: string | null
    targetRect?: PresenceTargetRect | null
    dwellBudgetMs?: number | null
  },
): void {
  const resolved = resolveSession(request, patch.body)
  if (!resolved) return
  const { sessionId, session } = resolved

  for (const [id, cursor] of presenceCursors) {
    if (cursor.clientName === session.clientName && id !== sessionId) {
      removePresenceCursor(id)
    }
  }

  const existing = presenceCursors.get(sessionId)
  const resolvedCanvasX = patch.canvasX ?? existing?.canvasX ?? 0
  const resolvedCanvasY = patch.canvasY ?? existing?.canvasY ?? 0
  const positionChanged =
    !existing ||
    existing.canvasX !== resolvedCanvasX ||
    existing.canvasY !== resolvedCanvasY
  const now = Date.now()
  const activeTask = activePresenceTasks.get(sessionId)
  const next: PresenceCursorEntry = {
    pageId: null,
    pageX: null,
    pageY: null,
    labelKey: null,
    labelParams: null,
    targetRef: null,
    targetRefSource: null,
    targetName: null,
    targetRect: null,
    dwellBudgetMs: null,
    ...pickDefined(existing, MERGED_CURSOR_FIELDS),
    ...pickDefined(patch, MERGED_CURSOR_FIELDS),
    sessionId,
    clientName: session.clientName,
    color: existing?.color ?? deriveColor(sessionId),
    canvasX: resolvedCanvasX,
    canvasY: resolvedCanvasY,
    surface: patch.surface ?? existing?.surface ?? 'canvas',
    activity: patch.activity ?? existing?.activity ?? 'acting',
    taskLabel:
      patch.taskLabel === undefined
        ? existing?.taskLabel ?? activeTask?.taskLabel ?? null
        : patch.taskLabel,
    labelHint:
      patch.labelHint === undefined
        ? existing?.labelHint ?? activeTask?.labelHint ?? null
        : patch.labelHint,
    updatedAt: now,
    lastMoveAt: positionChanged ? now : existing?.lastMoveAt ?? now,
    lastActAt: existing?.lastActAt ?? null,
    lastIntentLabelKey: withLastIntentLabelKey(existing, patch.labelKey),
  }

  presenceCursors.set(sessionId, next)
  schedulePresenceExpiry()
  notifyPresenceChanged()
}

/** The dwell budget (ms) the CDP proxy should pay before the next act on
 *  `sessionId` — burst-short if the session's last real dispatch landed
 *  inside the burst window, full otherwise (ADR 0029). Callers repositioning
 *  the cursor ahead of a dwell should stash this on the patch so the
 *  broadcast and the actual wait agree on the same number. */
export function computeDwellBudgetMs(sessionId: string | null | undefined): number {
  const cursor = sessionId ? presenceCursors.get(sessionId) : undefined
  const msSinceLastAct = cursor?.lastActAt != null ? Date.now() - cursor.lastActAt : null
  return selectDwellBudgetMs(msSinceLastAct)
}

/** Marks `sessionId`'s most recent real CDP dispatch (mouse press, scroll)
 *  — the regime signal `computeDwellBudgetMs` reads back on the next act.
 *  Mutates the cursor entry in place rather than going through
 *  `upsertPresenceCursor`, since this isn't a reposition or a re-tag. */
export function recordPresenceAct(sessionId: string | null | undefined): void {
  if (!sessionId) return
  const existing = presenceCursors.get(sessionId)
  if (!existing) return
  existing.lastActAt = Date.now()
}

export function upsertActivePresenceTask(
  request: IncomingMessage,
  patch: {
    body?: Record<string, unknown>
    taskLabel?: string | null
    surface?: PresenceSurface
    pageId?: string | null
    pageX?: number | null
    pageY?: number | null
    canvasX?: number | null
    canvasY?: number | null
    targetName?: string | null
    targetRect?: PresenceTargetRect | null
    labelHint?: string | null
  },
): void {
  const resolved = resolveSession(request, patch.body)
  if (!resolved) return
  const { sessionId, session } = resolved
  const existing = activePresenceTasks.get(sessionId)
  activePresenceTasks.set(sessionId, {
    taskLabel: null,
    pageId: null,
    pageX: null,
    pageY: null,
    canvasX: null,
    canvasY: null,
    targetName: null,
    targetRect: null,
    labelHint: null,
    ...pickDefined(existing, MERGED_TASK_FIELDS),
    ...pickDefined(patch, MERGED_TASK_FIELDS),
    sessionId,
    clientName: session.clientName,
    surface: patch.surface ?? existing?.surface ?? 'canvas',
    updatedAt: Date.now(),
  })
  schedulePresenceExpiry()
}

export function clearActivePresenceTask(
  request: IncomingMessage,
  body?: Record<string, unknown>,
): void {
  const resolved = resolveSession(request, body)
  if (!resolved) return
  beginPresenceDeparture(resolved.sessionId)
  schedulePresenceExpiry()
  notifyPresenceChanged()
}

// --- Timer and animation ---

export function scheduleThinkingState(request: IncomingMessage): void {
  if (thinkingTimer) clearTimeout(thinkingTimer)
  thinkingTimer = setTimeout(() => {
    thinkingTimer = null
    const resolved = resolveSession(request)
    if (!resolved) return
    const existing = presenceCursors.get(resolved.sessionId)
    if (!existing || existing.activity === 'idle') return
    const activeTask = activePresenceTasks.get(resolved.sessionId)
    if (activeTask) {
      activeTask.updatedAt = Date.now()
    }
    presenceCursors.set(resolved.sessionId, {
      ...existing,
      activity: 'thinking',
      labelKey: 'thinking',
      taskLabel: existing.taskLabel ?? activeTask?.taskLabel ?? null,
      labelHint: activeTask?.labelHint ?? existing.labelHint ?? null,
      updatedAt: Date.now(),
    })
    schedulePresenceExpiry()
    notifyPresenceChanged()
  }, PRESENCE_CURSOR_THINKING_DELAY_MS)
}

export function allEntityPositions(): Array<{ x: number; y: number }> {
  const positions: Array<{ x: number; y: number }> = []
  for (const page of pages) {
    positions.push({ x: page.canvasX, y: page.canvasY })
  }
  for (const te of getTextEntities()) {
    positions.push({ x: te.canvasX, y: te.canvasY })
  }
  for (const fe of getFileEntities()) {
    positions.push({ x: fe.canvasX, y: fe.canvasY })
  }
  positions.sort((a, b) => a.y - b.y || a.x - b.x)
  return positions
}

/** Look up the canvas position of any entity by id — page, text, file, or
 * group. Returns null if the id doesn't match anything. */
export function findEntityPosition(id: string): { x: number; y: number } | null {
  const page = pages.find((p) => p.id === id)
  if (page) return { x: page.canvasX, y: page.canvasY }
  const te = getTextEntities().find((e) => e.id === id)
  if (te) return { x: te.canvasX, y: te.canvasY }
  const fe = getFileEntities().find((e) => e.id === id)
  if (fe) return { x: fe.canvasX, y: fe.canvasY }
  const group = workspaceGroups.find((g) => g.id === id)
  if (group) return { x: group.canvasX, y: group.canvasY }
  return null
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function setPresenceCursorIdle(request: IncomingMessage): void {
  const resolved = resolveSession(request)
  if (!resolved) return
  const existing = presenceCursors.get(resolved.sessionId)
  if (!existing) return
  const activeTask = activePresenceTasks.get(resolved.sessionId)
  presenceCursors.set(resolved.sessionId, {
    ...existing,
    activity: activeTask ? 'thinking' : 'idle',
    labelKey: activeTask ? 'thinking' : 'idle',
    taskLabel: existing.taskLabel ?? activeTask?.taskLabel ?? null,
    labelHint: activeTask?.labelHint ?? existing.labelHint ?? null,
    updatedAt: Date.now(),
  })
  schedulePresenceExpiry()
  notifyPresenceChanged()
}

export function movePresenceCursorTo(
  request: IncomingMessage,
  canvasX: number,
  canvasY: number,
  labelKey: PresenceLabelKey | null,
): void {
  const resolved = resolveSession(request)
  if (!resolved) return
  const existing = presenceCursors.get(resolved.sessionId)
  if (!existing) return
  // Skip no-op moves
  if (existing.canvasX === canvasX && existing.canvasY === canvasY && existing.labelKey === labelKey) {
    return
  }
  const positionChanged = existing.canvasX !== canvasX || existing.canvasY !== canvasY
  const now = Date.now()
  const next: PresenceCursorEntry = {
    ...existing,
    canvasX,
    canvasY,
    surface: 'canvas',
    activity: 'traveling',
    labelKey,
    updatedAt: now,
    lastMoveAt: positionChanged ? now : existing.lastMoveAt,
    lastIntentLabelKey: withLastIntentLabelKey(existing, labelKey),
  }
  presenceCursors.set(resolved.sessionId, next)
  notifyPresenceChanged()
}

/** Stagger an operation across positions in the background. Cancellable via activeScanId. */
export function staggerOperation(
  request: IncomingMessage,
  items: Array<{ x: number; y: number }>,
  labelKey: PresenceLabelKey | null,
  perform: (index: number) => void,
): void {
  if (items.length === 0) return
  const scanId = bumpActiveScanId()
  void (async () => {
    for (let i = 0; i < items.length; i++) {
      if (activeScanId !== scanId) return
      movePresenceCursorTo(request, items[i].x, items[i].y, labelKey)
      await delay(PRESENCE_CURSOR_STEP_DELAY_MS)
      if (activeScanId !== scanId) return
      upsertPresenceCursor(request, {
        canvasX: items[i].x,
        canvasY: items[i].y,
        surface: 'canvas',
        activity: 'acting',
        labelKey,
      })
      perform(i)
    }
    setPresenceCursorIdle(request)
  })()
}

/** Animate cursor over positions without performing operations (for read scans). */
export function animateCursorScan(
  request: IncomingMessage,
  positions: Array<{ x: number; y: number }>,
  labelKey: PresenceLabelKey | null,
): void {
  staggerOperation(request, positions, labelKey, () => {})
}

// --- Reset ---

