// fallow-ignore-file circular-dependencies
// Suppressed: see #141. document-commands → surface-layout → canvas-layout-data import presence-cursor back
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
} from '../shared/presence-timing'
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
const PRESENCE_MOVE_LOGGING_ENABLED =
  process.env.NODE_ENV !== 'production' ||
  Boolean(process.env.MAIN_WINDOW_VITE_DEV_SERVER_URL)

// --- Coercion validation sets ---

const PRESENCE_LABEL_KEYS = new Set<PresenceLabelKey>([
  'scan_workspace',
  'find_placement',
  'create_page',
  'select_page',
  'attach_page',
  'inspect_page',
  'find_target',
  'click_target',
  'type_text',
  'select_option',
  'wait_page',
  'scroll_page',
  'read_content',
  'add_annotation',
  'thinking',
  'idle',
])

const PRESENCE_ACTIVITIES = new Set<PresenceActivity>([
  'traveling',
  'acting',
  'waiting',
  'thinking',
  'idle',
  'departing',
])

const PRESENCE_SURFACES = new Set<PresenceSurface>(['canvas', 'page'])

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
  return typeof value === 'string' && PRESENCE_LABEL_KEYS.has(value as PresenceLabelKey)
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

function formatCoord(value: number | null | undefined): string {
  return typeof value === 'number' ? value.toFixed(1) : '-'
}

function formatTargetRect(rect: PresenceTargetRect | null | undefined): string {
  if (!rect) return '-'
  return `${rect.x.toFixed(1)},${rect.y.toFixed(1)},${rect.width.toFixed(1)},${rect.height.toFixed(1)}`
}

function coordSourceForLog(
  existing: PresenceCursorEntry | undefined,
  patch: {
    surface?: PresenceSurface
    pageX?: number | null
    pageY?: number | null
    targetRect?: PresenceTargetRect | null
    canvasX?: number
    canvasY?: number
  },
): string {
  const surface = patch.surface ?? existing?.surface ?? 'canvas'
  if (surface !== 'page') return 'canvas'
  if (typeof patch.pageX === 'number' && typeof patch.pageY === 'number') {
    return 'page-point'
  }
  if (patch.targetRect) return 'target-rect'
  if (typeof patch.canvasX === 'number' && typeof patch.canvasY === 'number') {
    return 'canvas-only'
  }
  return 'existing'
}

function logPresenceMove(
  request: IncomingMessage,
  source: 'upsertPresenceCursor' | 'movePresenceCursorTo',
  sessionId: string,
  clientName: string,
  existing: PresenceCursorEntry | undefined,
  next: PresenceCursorEntry,
  patch: {
    body?: Record<string, unknown>
    canvasX?: number
    canvasY?: number
    pageX?: number | null
    pageY?: number | null
    targetRect?: PresenceTargetRect | null
  },
): void {
  if (!PRESENCE_MOVE_LOGGING_ENABLED) return
  const changed =
    !existing ||
    existing.canvasX !== next.canvasX ||
    existing.canvasY !== next.canvasY
  if (!changed) return
  if (!existing && patch.canvasX === undefined && patch.canvasY === undefined) return

  const body = patch.body ?? {}
  const route = `${request.method ?? 'GET'} ${request.url ?? ''}`.trim()
  const eventType =
    typeof body.eventType === 'string' ? body.eventType : null
  const command = typeof body.command === 'string' ? body.command : null
  const coordSource = coordSourceForLog(existing, patch)
  const suspiciousCenterFallback =
    next.surface === 'page' &&
    coordSource === 'canvas-only' &&
    typeof next.pageX !== 'number' &&
    typeof next.pageY !== 'number' &&
    !next.targetRect
  const prevX = existing?.canvasX
  const prevY = existing?.canvasY
  const dx =
    typeof prevX === 'number' ? (next.canvasX - prevX).toFixed(1) : 'n/a'
  const dy =
    typeof prevY === 'number' ? (next.canvasY - prevY).toFixed(1) : 'n/a'

  const parts = [
    '[presence-move]',
    `source=${source}`,
    `route=${JSON.stringify(route)}`,
    `session=${sessionId.slice(0, 8)}`,
    `client=${JSON.stringify(clientName)}`,
    `from=(${formatCoord(prevX)},${formatCoord(prevY)})`,
    `to=(${formatCoord(next.canvasX)},${formatCoord(next.canvasY)})`,
    `delta=(${dx},${dy})`,
    `surface=${next.surface}`,
    `activity=${next.activity}`,
    `label=${next.labelKey ?? '-'}`,
    `coordSource=${coordSource}`,
    `page=${next.pageId ?? '-'}`,
    `pagePoint=(${formatCoord(next.pageX)},${formatCoord(next.pageY)})`,
    `targetRect=${formatTargetRect(next.targetRect)}`,
    `targetRef=${next.targetRef ?? '-'}`,
    `targetName=${JSON.stringify(next.targetName ?? '-')}`,
  ]
  if (eventType) parts.push(`event=${eventType}`)
  if (command) parts.push(`command=${command}`)
  if (suspiciousCenterFallback) parts.push('suspect=center-fallback')

  console.log(parts.join(' '))
}

// --- Cursor mutation functions ---

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
  const next: PresenceCursorEntry = {
    sessionId,
    clientName: session.clientName,
    color: existing?.color ?? deriveColor(sessionId),
    canvasX: resolvedCanvasX,
    canvasY: resolvedCanvasY,
    surface: patch.surface ?? existing?.surface ?? 'canvas',
    activity: patch.activity ?? existing?.activity ?? 'acting',
    pageId:
      patch.pageId === undefined
        ? existing?.pageId ?? null
        : patch.pageId,
    pageX:
      patch.pageX === undefined
        ? existing?.pageX ?? null
        : patch.pageX,
    pageY:
      patch.pageY === undefined
        ? existing?.pageY ?? null
        : patch.pageY,
    labelKey:
      patch.labelKey === undefined
        ? existing?.labelKey ?? null
        : patch.labelKey,
    taskLabel:
      patch.taskLabel === undefined
        ? existing?.taskLabel ?? activePresenceTasks.get(sessionId)?.taskLabel ?? null
        : patch.taskLabel,
    labelHint:
      patch.labelHint === undefined
        ? existing?.labelHint ?? activePresenceTasks.get(sessionId)?.labelHint ?? null
        : patch.labelHint,
    labelParams:
      patch.labelParams === undefined
        ? existing?.labelParams ?? null
        : patch.labelParams,
    targetRef:
      patch.targetRef === undefined
        ? existing?.targetRef ?? null
        : patch.targetRef,
    targetRefSource:
      patch.targetRefSource === undefined
        ? existing?.targetRefSource ?? null
        : patch.targetRefSource,
    targetName:
      patch.targetName === undefined
        ? existing?.targetName ?? null
        : patch.targetName,
    targetRect:
      patch.targetRect === undefined
        ? existing?.targetRect ?? null
        : patch.targetRect,
    updatedAt: now,
    lastMoveAt: positionChanged ? now : existing?.lastMoveAt ?? now,
  }

  presenceCursors.set(sessionId, next)
  logPresenceMove(
    request,
    'upsertPresenceCursor',
    sessionId,
    session.clientName,
    existing,
    next,
    patch,
  )

  schedulePresenceExpiry()
  notifyPresenceChanged()
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
    sessionId,
    clientName: session.clientName,
    taskLabel:
      patch.taskLabel === undefined
        ? existing?.taskLabel ?? null
        : patch.taskLabel,
    surface: patch.surface ?? existing?.surface ?? 'canvas',
    pageId:
      patch.pageId === undefined
        ? existing?.pageId ?? null
        : patch.pageId,
    pageX:
      patch.pageX === undefined
        ? existing?.pageX ?? null
        : patch.pageX,
    pageY:
      patch.pageY === undefined
        ? existing?.pageY ?? null
        : patch.pageY,
    canvasX:
      patch.canvasX === undefined
        ? existing?.canvasX ?? null
        : patch.canvasX,
    canvasY:
      patch.canvasY === undefined
        ? existing?.canvasY ?? null
        : patch.canvasY,
    targetName:
      patch.targetName === undefined
        ? existing?.targetName ?? null
        : patch.targetName,
    targetRect:
      patch.targetRect === undefined
        ? existing?.targetRect ?? null
        : patch.targetRect,
    labelHint:
      patch.labelHint === undefined
        ? existing?.labelHint ?? null
        : patch.labelHint,
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
  }
  presenceCursors.set(resolved.sessionId, next)
  logPresenceMove(
    request,
    'movePresenceCursorTo',
    resolved.sessionId,
    resolved.session.clientName,
    existing,
    next,
    { canvasX, canvasY },
  )
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

