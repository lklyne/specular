import type { WebContents } from 'electron'
import type {
  CanvasHoverTarget,
  CanvasInteractionState,
  DevtoolsPanelData,
  FocusPresentationMode,
  InspectNodeDetail,
} from '../../shared/types'
import type { FocusTarget } from '../../shared/interaction-types'
import type { Page } from './runtime-entities'

// --- Viewport state ---

export let zoom = 1.0
export let pan = { x: 0, y: 0 }

export function setZoomState(value: number): void {
  zoom = value
}

export function getZoom(): number {
  return zoom
}

export function setPanState(value: { x: number; y: number }): void {
  pan = value
}

// --- Canvas interaction flags ---

export let selectionOverlayActive = false
export let hoverTarget: CanvasHoverTarget = null
export let interactionState: CanvasInteractionState = { kind: 'idle' }
export let arrowNavigationLocked = false
export let spaceModifierHeld = false
export let hoveringCanvasChrome = false

/**
 * Single runtime variable for "which entity is currently being edited
 * inline on the canvas." Derived from `interactionState` — populated
 * while the controller is in `editing-entity` mode, null otherwise. The
 * IPC layer uses `beginEditingEntity` / `commitEditingEntity` /
 * `cancelEditingEntity` (see `editing-entity-runtime.ts`) to drive the
 * mode transition; reads can use this getter or inspect the
 * broadcast `interaction` field directly.
 */
export function getEditingEntityId(): string | null {
  return interactionState.kind === 'editing-entity'
    ? interactionState.entityId
    : null
}

// Explicit focus intent consumed by FocusReconciler at end of the next
// layout pass. Null means "derive from state". Cleared after reconcile.
export let pendingFocus: FocusTarget | null = null

export function setPendingFocus(value: FocusTarget | null): void {
  pendingFocus = value
}

export function setSelectionOverlayActive(value: boolean): void {
  selectionOverlayActive = value
}

export function setHoverTarget(value: CanvasHoverTarget): void {
  hoverTarget = value
}

export function setInteractionState(value: CanvasInteractionState): void {
  if (
    value.kind !== 'idle' &&
    interactionState.kind !== 'idle' &&
    value.kind !== interactionState.kind
  ) {
    console.warn('[interaction-state] ignored conflicting transition', {
      current: interactionState,
      next: value,
    })
    return
  }
  interactionState = value
}

export function setArrowNavigationLocked(value: boolean): void {
  arrowNavigationLocked = value
}

export function setSpaceModifierHeld(value: boolean): void {
  spaceModifierHeld = value
}

export function setHoveringCanvasChrome(value: boolean): void {
  hoveringCanvasChrome = value
}

// --- Focus presentation ---

export type FocusPresentationOverride = {
  pageId: string
  mode: FocusPresentationMode
}

export let focusPresentationOverride: FocusPresentationOverride | null = null

export function setFocusPresentationOverride(value: FocusPresentationOverride | null): void {
  focusPresentationOverride = value
}

// Wall-clock (Date.now) start of the in-flight animated camera move, or null
// when idle. Broadcast in layout so renderer flip animations (popup morph) can
// fast-forward into phase with the main-driven camera instead of starting late.
export let cameraTransitionStartedAt: number | null = null

export function setCameraTransitionStartedAt(value: number | null): void {
  cameraTransitionStartedAt = value
}

// --- Inspect state ---

export let inspectActivePageId: string | null = null
export let inspectHoveredTarget: InspectNodeDetail | null = null
export let inspectSelectedTarget: InspectNodeDetail | null = null
export const inspectSelectedNodeIdByPage = new Map<string, string>()

export function setInspectActivePageId(value: string | null): void {
  inspectActivePageId = value
}

export function setInspectHoveredTarget(value: InspectNodeDetail | null): void {
  inspectHoveredTarget = value
}

export function setInspectSelectedTarget(value: InspectNodeDetail | null): void {
  inspectSelectedTarget = value
}

// --- Workspace persistence ---

export let workspaceAutosaveTimer: NodeJS.Timeout | null = null
export let workspacePersistenceSuspendCount = 0

export function setWorkspaceAutosaveTimer(value: NodeJS.Timeout | null): void {
  workspaceAutosaveTimer = value
}

export function incrementWorkspacePersistenceSuspendCount(): void {
  workspacePersistenceSuspendCount += 1
}

export function decrementWorkspacePersistenceSuspendCount(): void {
  workspacePersistenceSuspendCount = Math.max(0, workspacePersistenceSuspendCount - 1)
}

// --- MCP / DevTools state ---

export let browserDevtoolsAttachGeneration = 0
export const automationInteractivePageCounts = new Map<string, number>()
export let mcpConnectionStatus: NonNullable<
  NonNullable<DevtoolsPanelData['emptyState']>['status']
> = {
  healthy: false,
  appServerRunning: false,
  discoveryFilePresent: false,
  mcpClientConnected: false,
  activeClientCount: 0,
  lastClientSeenAt: null,
}

export function incrementBrowserDevtoolsAttachGeneration(): number {
  browserDevtoolsAttachGeneration += 1
  return browserDevtoolsAttachGeneration
}

export function addAutomationInteractivePageId(pageId: string): void {
  automationInteractivePageCounts.set(pageId, (automationInteractivePageCounts.get(pageId) ?? 0) + 1)
}

export function removeAutomationInteractivePageId(pageId: string): void {
  const current = automationInteractivePageCounts.get(pageId) ?? 0
  if (current <= 1) {
    automationInteractivePageCounts.delete(pageId)
    return
  }
  automationInteractivePageCounts.set(pageId, current - 1)
}

export function clearAutomationInteractivePageIds(): void {
  automationInteractivePageCounts.clear()
}

export function setMcpConnectionStatusState(
  value: NonNullable<NonNullable<DevtoolsPanelData['emptyState']>['status']>,
): void {
  mcpConnectionStatus = value
}

// --- Pages ---

export const pages: Page[] = []

import { selectedPageIndex as uiSelectedPageIndex } from '../ui-state'

export function findPageById(id: string): Page | undefined {
  return pages.find((page) => page.id === id)
}

export function findPageByPageView(senderWebContents: WebContents): Page | undefined {
  return pages.find((p) => p.pageView.webContents === senderWebContents)
}

export function selectedPage(): Page | null {
  const selectedIndex = uiSelectedPageIndex(pages.map((p) => p.id))
  if (selectedIndex === null || selectedIndex < 0 || selectedIndex >= pages.length) return null
  return pages[selectedIndex]
}

export function selectedPageId(): string | null {
  return selectedPage()?.id ?? null
}
