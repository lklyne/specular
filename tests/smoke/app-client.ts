import { readFileSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import type { CanvasGuidesPayload } from '../../src/shared/canvas-guides'
import type { JsonCanvasDocument } from '../../src/shared/json-canvas-types'

function loadEnv(): { port: number; secret: string } {
  const raw = readFileSync(join(tmpdir(), 'specular-smoke-env.json'), 'utf8')
  return JSON.parse(raw)
}

let _env: { port: number; secret: string } | null = null
function env() {
  if (!_env) _env = loadEnv()
  return _env
}

const baseUrl = () => `http://127.0.0.1:${env().port}`
const secret = () => env().secret

function headers(): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    'X-Specular-Secret': secret(),
  }
}

function headersWithSession(sessionId?: string, clientName?: string): Record<string, string> {
  return {
    ...headers(),
    ...(sessionId ? { 'X-Specular-Session-Id': sessionId } : {}),
    ...(clientName ? { 'X-Specular-Client-Name': clientName } : {}),
  }
}

async function get<T = unknown>(path: string): Promise<T> {
  const res = await fetch(`${baseUrl()}${path}`, { headers: headers() })
  if (!res.ok) throw new Error(`GET ${path} → ${res.status}: ${await res.text()}`)
  return res.json() as Promise<T>
}

async function getWithSession<T = unknown>(path: string, sessionId?: string, clientName?: string): Promise<T> {
  const res = await fetch(`${baseUrl()}${path}`, {
    headers: headersWithSession(sessionId, clientName),
  })
  if (!res.ok) throw new Error(`GET ${path} → ${res.status}: ${await res.text()}`)
  return res.json() as Promise<T>
}

async function post<T = unknown>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${baseUrl()}${path}`, {
    method: 'POST',
    headers: headers(),
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  if (!res.ok) throw new Error(`POST ${path} → ${res.status}: ${await res.text()}`)
  return res.json() as Promise<T>
}

async function del<T = unknown>(path: string): Promise<T> {
  const res = await fetch(`${baseUrl()}${path}`, {
    method: 'DELETE',
    headers: headers(),
  })
  if (!res.ok) throw new Error(`DELETE ${path} → ${res.status}: ${await res.text()}`)
  return res.json() as Promise<T>
}

// --- Health ---

export function getHealth() {
  return get<{ version: string }>('/health')
}

export function getCdpProxyDebug() {
  return get<{
    registrations: Array<{
      token: string
      pageId: string
      sessionId: string | null
      status: string
      hasActiveClient: boolean
    }>
    metrics: {
      registrationsCreated: number
      registrationsReused: number
      upstreamConnects: number
      upstreamReconnects: number
      interceptedClicks: number
      interceptedScrolls: number
    }
  }>('/debug/cdp-proxy')
}

// --- Workspace ---

export function getWorkspace() {
  return get<{ entities: { id: string; kind: string }[]; edges: unknown[]; selection: unknown; camera: unknown }>('/workspace')
}

export function getSidebar() {
  type SidebarItem = {
    kind: 'page' | 'text' | 'file' | 'group' | 'drawing' | 'shape'
    id: string
    label: string
    children?: SidebarItem[]
    entityCount?: number
  }
  return get<{
    sections: { notes: SidebarItem[]; pages: SidebarItem[] }
    items: SidebarItem[]
  }>('/sidebar')
}

export function getEntityOrder() {
  return get<{ entityOrder: string[] }>('/test/workspace/entity-order')
}

export function loadCanvasFixture(input: { name?: string; doc: JsonCanvasDocument }) {
  return post<{ ok: true; entityOrder: string[] }>('/test/workspace/load-canvas-fixture', input)
}

export function reorderSidebarItem(input: {
  section: 'notes' | 'pages'
  draggedId: string
  anchorId?: string | null
  position?: 'before' | 'after'
  parentId?: string | null
}) {
  return post<{ ok: boolean; entityOrder: string[] }>('/test/sidebar/reorder', input)
}

export type StackOrderAction =
  | 'bring-forward'
  | 'send-backward'
  | 'bring-to-front'
  | 'send-to-back'

export function reorderStackOrder(
  action: StackOrderAction,
  input: { id: string } | { ids: string[] },
) {
  return post<{ ok: boolean; entityOrder: string[] }>(`/stack-order/${action}`, input)
}

// --- Pages ---

export function createPages(pages: { url: string; presetIndex?: number; canvasX?: number; canvasY?: number }[]) {
  return post<{ pageIds: string[] }>('/pages/create', { pages })
}

export function createFocusedPage(input: {
  sourcePageId?: string
  presetIndex?: number
  canvasX: number
  canvasY: number
}) {
  return post<{ pageId: string }>('/pages/create-at-position', input)
}

export function deletePages(pageIds: string[]) {
  return post<{ deletedPageIds: string[] }>('/pages/delete', { pageIds })
}

function updatePages(pages: Array<{ id: string; canvasX?: number; canvasY?: number; presetIndex?: number }>) {
  return post<{ updated: string[] }>('/pages/update', { pages })
}

export function applyLayoutDirective(body: {
  layout: {
    kind: 'row' | 'column' | 'grid'
    gap?: number | string
    rowGap?: number | string
    colGap?: number | string
    cols?: number
    originX?: number
    originY?: number
    near?: string
  }
  items: Array<{
    id?: string
    width?: number
    height?: number
    insetX?: number
    insetY?: number
  }>
}) {
  return post<{
    positions: Array<{ canvasX: number; canvasY: number }>
    kinds: Array<string | null>
    warnings?: string[]
  }>('/layout/apply-directive', body)
}

export function takeScreenshot(pageId?: string) {
  return post<{ base64: string; mimeType: string }>('/pages/screenshot', { pageId })
}

export function getPageCdpTarget(pageId: string, sessionId?: string, clientName?: string) {
  return getWithSession<{
    pageId: string
    targetId: string
    webSocketDebuggerUrl: string
    url: string
    title: string
  }>(`/pages/${encodeURIComponent(pageId)}/cdp-target`, sessionId, clientName)
}

export function takeSnapshot(pageId?: string) {
  return post<{ snapshot: string }>('/pages/snapshot', { pageId })
}

export function takeAgentSnapshot(pageId?: string) {
  return post<{ snapshot: { pageId: string; url: string; title: string; nodes: Array<{
    ref: string
    depth: number
    tagName: string
    interactive: boolean
    bounds: { x: number; y: number; width: number; height: number }
  }> } }>('/pages/agent-snapshot', { pageId })
}

export function getPresence() {
  return get<{ cursors: Array<{
    sessionId: string
    clientName: string
    surface?: 'canvas' | 'page'
    activity?: 'traveling' | 'acting' | 'waiting' | 'thinking' | 'idle'
    pageId?: string | null
    pageX?: number | null
    pageY?: number | null
    labelKey?: string | null
    taskLabel?: string | null
    labelHint?: string | null
    targetRef?: string | null
    targetRefSource?: string | null
    targetName?: string | null
    targetRect?: { x: number; y: number; width: number; height: number } | null
  }> }>('/session/presence')
}

export function postPresence(body: unknown) {
  return post<{ ok: true }>('/session/presence', body)
}

export function openMcpSession(sessionId: string, clientName?: string) {
  return post<{ ok: true }>('/mcp/session/open', { sessionId, clientName })
}

function pingMcpSession(sessionId: string, clientName?: string) {
  return post<{ ok: true }>('/mcp/session/ping', { sessionId, clientName })
}

export function closeMcpSession(sessionId: string) {
  return post<{ ok: true }>('/mcp/session/close', { sessionId })
}

export function resetSmokeState() {
  return post<{ ok: true }>('/test/reset-state')
}

export function findPageTarget(body: unknown) {
  return post<{ target: {
    targetRef?: string | null
    targetRefSource?: string | null
    targetName?: string | null
    targetRect: { x: number; y: number; width: number; height: number }
    pageX: number
    pageY: number
  } }>('/pages/find-target', body)
}

// --- Text entities ---

export function getTextEntities() {
  return get<{
    textEntities: Array<{
      id: string
      text: string
      color?: string
      canvasX: number
      canvasY: number
      width: number
      height: number
    }>
  }>('/text-entities')
}

export async function createTextEntities(items: {
  canvasX: number
  canvasY: number
  text?: string
  color?: string
  width?: number
  height?: number
}[]): Promise<{ ids: string[] }> {
  if (items.length === 1) {
    // Single item returns TextEntity directly
    const entity = await post<{ id: string }>('/text-entities/create', items[0])
    return { ids: [entity.id] }
  }
  const result = await post<{ items: { id: string }[] }>('/text-entities/create', { items })
  return { ids: result.items.map((i) => i.id) }
}

export function updateTextEntities(items: { id: string; patch: { text?: string; color?: string; canvasX?: number; canvasY?: number } }[]) {
  return post<{ items: { id: string }[] }>('/text-entities/update', { items })
}

export function deleteTextEntities(ids: string[]) {
  return post<{ deleted: string[] }>('/text-entities/delete', { ids })
}

// --- Edges ---

export function createEdges(edges: Array<{
  id?: string
  fromEntityId: string
  toEntityId: string
  kind: 'breakpoint_variant' | 'connection'
}>) {
  return post<{ edgeIds: string[] }>('/edges/create', { edges })
}

export function deleteEdges(edgeIds: string[]) {
  return post<{ deletedEdgeIds: string[] }>('/edges/delete', { edgeIds })
}

// --- Selection ---

export function getSelection() {
  return get<{ selectedEntityId?: string; selectedEntityIds?: string[]; selectedGroupId?: string }>('/selection')
}

export function getSelectionOverlayState() {
  return get<{ pages: { pageId: string; interactive: boolean; multiSelected: boolean }[] }>('/selection/overlay-state')
}

export function deselectSelection() {
  return post('/selection/deselect')
}

export function selectPage(pageId: string) {
  return post<{ ok: boolean; selection: { selectedEntityId?: string; selectedEntityIds?: string[]; selectedGroupId?: string } }>(
    '/selection/select-page',
    { pageId },
  )
}

export function selectEntity(entityId: string, entityKind: 'page' | 'text' | 'file' | 'edge') {
  return post<{ ok: true; selection: { selectedEntityId?: string; selectedEntityIds?: string[]; selectedGroupId?: string } }>(
    '/selection/select-entity',
    { entityId, entityKind },
  )
}

export function selectEntities(entityIds: string[]) {
  return post('/selection/select-entities', { entityIds })
}

export function selectGroup(groupId: string) {
  return post('/selection/select-group', { groupId })
}

export function enterGroup(groupId: string) {
  return post('/selection/enter-group', { groupId })
}

export function createGroup(entityIds: string[], label?: string) {
  return post<{ id: string; entityIds: string[] }>('/groups/create', { entityIds, label })
}

export function ungroup(groupId: string) {
  return post<{ entityIds: string[] }>('/groups/ungroup', { groupId })
}

export function deleteGroups(groupIds: string[]) {
  return post<{ deletedGroupIds: string[] }>('/groups/delete', { groupIds })
}

// --- Camera ---

export function focusCamera(targets: { pageIds?: string[]; bounds?: { x: number; y: number; width: number; height: number } }) {
  return post('/camera/focus', targets)
}

// --- Test-only: interaction controller, focus, drop ---

type InteractionMode =
  | { kind: 'idle' }
  | { kind: 'panning' }
  | { kind: 'marquee'; origin: { x: number; y: number }; current: { x: number; y: number } }
  | { kind: 'dragging-entities'; ids: string[]; anchor: { x: number; y: number } }
  | { kind: 'resizing-entity'; id: string; edge: string }
  | { kind: 'resizing-multi-selection' }
  | { kind: 'dragging-edge'; from: unknown; target: unknown }
  | { kind: 'editing-entity'; id: string }

export type InteractionToken = { id: string; mode: string }
export type CancelReason = 'blur' | 'escape' | 'undo' | 'tab-switch' | 'external'

export type TryEnterInput =
  | { kind: 'panning' }
  | { kind: 'marquee' }
  | { kind: 'dragging-entities'; entityIds: string[] }
  | { kind: 'resizing-entity'; target: { kind: string; id: string } }
  | { kind: 'resizing-multi-selection' }
  | { kind: 'editing-entity'; entityId: string }
  | { kind: 'dragging-edge'; from: { kind: string; id: string }; fromSide: 'top' | 'right' | 'bottom' | 'left' }

export function getInteractionMode() {
  return get<{ mode: InteractionMode; editingEntityId: string | null }>(
    '/test/interaction/mode',
  )
}

export async function beginInteraction(input: TryEnterInput): Promise<InteractionToken | { refused: true; reason: string }> {
  const res = await fetch(`${baseUrl()}/test/interaction/begin`, {
    method: 'POST',
    headers: headers(),
    body: JSON.stringify(input),
  })
  const data = (await res.json()) as { token?: InteractionToken; refused?: true; reason?: string }
  if (res.status === 409 && data.refused) {
    return { refused: true, reason: data.reason ?? 'unknown' }
  }
  if (!res.ok) throw new Error(`POST /test/interaction/begin → ${res.status}`)
  return data.token!
}

export function commitInteraction(token: InteractionToken) {
  return post<{ ok: true; mode: InteractionMode }>('/test/interaction/commit', { token })
}

export function cancelInteraction(token: InteractionToken, reason: CancelReason) {
  return post<{ ok: true; mode: InteractionMode }>('/test/interaction/cancel', { token, reason })
}

export function cancelActiveInteraction(reason: CancelReason) {
  return post<{ ok: true; mode: InteractionMode }>('/test/interaction/cancel-active', { reason })
}

export function resetInteraction() {
  return post<{ ok: true }>('/test/interaction/reset')
}

type FocusKey = 'bgView' | 'aboveView' | 'toolbar' | 'sidebar' | string | null
export type FocusTarget =
  | { kind: 'bgView' }
  | { kind: 'aboveView' }
  | { kind: 'toolbar' }
  | { kind: 'sidebar' }
  | { kind: 'page'; id: string }

export function getCurrentFocus() {
  return get<{ focused: FocusKey }>('/test/focus/current')
}

export function requestFocus(target: FocusTarget) {
  return post<{ ok: true }>('/test/focus/request', { target })
}

export function consumeDragId(dragId: string) {
  return post<{ wasConsumed: boolean }>('/test/drop/consume-drag-id', { dragId })
}

export function resetDropOwner() {
  return post<{ ok: true }>('/test/drop/reset')
}

export function startCanvasDrag(entityIds: string[]) {
  return post<{ ok: true }>('/test/canvas-drag/start', { entityIds })
}

export function applyCanvasDrag(input: {
  entityIds: string[]
  dx: number
  dy: number
  shiftKey?: boolean
}) {
  return post<{ ok: true; guides: CanvasGuidesPayload }>('/test/canvas-drag/apply', input)
}

export function endCanvasDrag() {
  return post<{ ok: true; guides: CanvasGuidesPayload }>('/test/canvas-drag/end')
}

export function getCanvasGuides() {
  return get<CanvasGuidesPayload>('/test/canvas-guides/current')
}

export type MultiResizeEntry = {
  id: string
  kind: 'page' | 'text' | 'file' | 'drawing' | 'shape'
  width: number
  height: number
  canvasX: number
  canvasY: number
}

export function beginMultiResize() {
  return post<{ ok: true }>('/test/canvas-multi-resize/begin')
}

export function applyMultiResize(entries: MultiResizeEntry[]) {
  return post<{ ok: true }>('/test/canvas-multi-resize/apply', { entries })
}

export function endMultiResize() {
  return post<{ ok: true }>('/test/canvas-multi-resize/end')
}

// --- Tool state ---

export function getCurrentTool() {
  return get<{ tool: { kind: string } }>('/test/tool/current')
}

// --- Keyboard simulation ---

export function sendKey(
  key: string,
  options: {
    cmd?: boolean
    shift?: boolean
    alt?: boolean
    target?: 'aboveView' | 'bgView' | 'toolbar' | 'page'
    pageId?: string
  } = {},
) {
  return post<{ ok: true }>('/test/keyboard/send', { key, ...options })
}

export function pasteClipboardText(input: { text: string; canvasX?: number; canvasY?: number }) {
  return post<{ ok: true }>('/test/clipboard/paste', input)
}

// --- Workspace undo/redo and persistence (test-only) ---

export function undoWorkspace() {
  return post<{ ok: true; canUndo: boolean; canRedo: boolean }>(
    '/test/workspace/undo',
  )
}

export function redoWorkspace() {
  return post<{ ok: true; canUndo: boolean; canRedo: boolean }>(
    '/test/workspace/redo',
  )
}

export function getUndoState() {
  return get<{ canUndo: boolean; canRedo: boolean }>('/test/workspace/undo-state')
}

export function flushWorkspaceAutosave() {
  return post<{ ok: true }>('/test/workspace/flush-autosave')
}

export type DiskSnapshot = {
  exists: boolean
  meta: {
    activeTabId: string
    viewMode?: string
    tabs: Array<{ id: string; name: string; updatedAt: string; expanded?: boolean }>
  } | null
  tab: { id: string; name: string; updatedAt: string; expanded?: boolean } | null
  filePath?: string
  doc?: {
    nodes: Array<{
      id: string
      type: string
      x?: number
      y?: number
      width?: number
      height?: number
      [key: string]: unknown
    }>
    edges: Array<{ id: string; fromNode: string; toNode: string }>
    [key: string]: unknown
  } | null
}

export function getDiskSnapshot(tabId?: string) {
  const q = tabId ? `?tabId=${encodeURIComponent(tabId)}` : ''
  return get<DiskSnapshot>(`/test/workspace/disk-snapshot${q}`)
}

export function startTransactionCounter() {
  return post<{ ok: true }>('/test/workspace/transactions/start')
}

export function stopTransactionCounter() {
  return post<{ count: number }>('/test/workspace/transactions/stop')
}

function getWorkspaceTabs() {
  return get<{ activeTabId: string | null; tabs: Array<{ id: string; name: string }> }>(
    '/test/workspace/tabs',
  )
}

function getFileEntities() {
  return get<{
    fileEntities: Array<{
      id: string
      file: string
      width: number
      height: number
      canvasX: number
      canvasY: number
    }>
  }>('/file-entities')
}
