import type {
  CanvasEntityKind,
  CanvasSelectableTarget,
  DevtoolsPanelTab,
  Tool,
  UiState,
} from '../shared/types'
import { markDirty } from './runtime/layout-dirty'

type SelectionInput =
  | { kind: 'none' }
  | { kind: 'single-entity'; entityId: string; entityKind: CanvasEntityKind }
  | {
      kind: 'multi-entity'
      entityIds: string[]
      entityKindsById: Partial<Record<string, CanvasEntityKind>>
    }

const DEFAULT_DEVTOOLS_WIDTH = 400

let uiState: UiState = createDefaultUiState()

function sanitizeSelectionInput(input: SelectionInput): SelectionInput {
  if (input.kind === 'multi-entity') {
    const entityKindsById = Object.fromEntries(
      input.entityIds.map((entityId) => [entityId, input.entityKindsById[entityId] ?? 'page']),
    ) as Partial<Record<string, CanvasEntityKind>>
    return { kind: 'multi-entity', entityIds: input.entityIds, entityKindsById }
  }

  return input
}

export function createDefaultUiState(): UiState {
  return {
    selection: { kind: 'none' },
    activeTool: { kind: 'select' },
    leftSidebarOpen: true,
    toolbarDropdownOpen: false,
    devtools: {
      open: false,
      activeTab: 'comments',
      focusedAnnotationId: null,
      width: DEFAULT_DEVTOOLS_WIDTH,
    },
    overlays: {
      commentOverlayVisible: false,
      selectionMarqueeVisible: false,
    },
  }
}

export function resetUiState(next?: Partial<UiState>): UiState {
  uiState = {
    ...createDefaultUiState(),
    ...next,
  }
  return getUiState()
}

export function getUiState(): UiState {
  return cloneUiState(uiState)
}

export function replaceUiState(nextState: UiState): UiState {
  uiState = cloneUiState(nextState)
  setSelection(uiState.selection)
  return getUiState()
}

export function setSelection(input: SelectionInput): UiState {
  const nextInput = sanitizeSelectionInput(input)
  if (nextInput.kind === 'single-entity') {
    uiState.selection = { kind: 'single-entity', entityId: nextInput.entityId, entityKind: nextInput.entityKind }
    markDirty('canvas', 'sidebar', 'toolbar')
    return getUiState()
  }
  if (nextInput.kind === 'multi-entity') {
    const uniqueIds = [...new Set(nextInput.entityIds)]
    if (uniqueIds.length <= 1) {
      const entityId = uniqueIds[0]
      uiState.selection =
        entityId !== undefined
          ? {
              kind: 'single-entity',
              entityId,
              entityKind: nextInput.entityKindsById[entityId] ?? 'page',
            }
          : { kind: 'none' }
    } else {
      uiState.selection = {
        kind: 'multi-entity',
        entityIds: uniqueIds,
        entityKindsById: { ...nextInput.entityKindsById },
      }
    }
    markDirty('canvas', 'sidebar', 'toolbar')
    return getUiState()
  }
  uiState.selection = { kind: 'none' }
  markDirty('canvas', 'sidebar', 'toolbar')
  return getUiState()
}

export function setActiveTool(tool: Tool): UiState {
  uiState.activeTool = tool
  markDirty('canvas', 'toolbar')
  return getUiState()
}

export function updateSelectionForRemovedEntity(entityId: string): UiState {
  if (uiState.selection.kind === 'single-entity') {
    if (uiState.selection.entityId === entityId) {
      uiState.selection = { kind: 'none' }
    }
  } else if (uiState.selection.kind === 'multi-entity') {
    const { entityIds, entityKindsById } = uiState.selection
    const remaining = entityIds.filter((candidate) => candidate !== entityId)
    const remainingKindsById = Object.fromEntries(
      remaining.map((id) => [id, entityKindsById[id] ?? 'page']),
    ) as Partial<Record<string, CanvasEntityKind>>
    uiState.selection =
      remaining.length > 1
        ? {
            kind: 'multi-entity',
            entityIds: remaining,
            entityKindsById: remainingKindsById,
          }
        : remaining[0]
          ? {
              kind: 'single-entity',
              entityId: remaining[0],
              entityKind: remainingKindsById[remaining[0]] ?? 'page',
            }
          : { kind: 'none' }
  }

  return getUiState()
}

export function setDevtoolsOpen(open: boolean): UiState {
  uiState.devtools.open = open
  return getUiState()
}

export function setLeftSidebarOpen(open: boolean): UiState {
  uiState.leftSidebarOpen = open
  markDirty('toolbar')
  return getUiState()
}

export function setToolbarDropdownOpen(open: boolean): UiState {
  uiState.toolbarDropdownOpen = open
  return getUiState()
}

export function setDevtoolsPanelTab(tab: DevtoolsPanelTab): UiState {
  uiState.devtools.activeTab = tab
  if (tab !== 'comments') {
    uiState.devtools.focusedAnnotationId = null
  }
  return getUiState()
}

export function focusAnnotation(annotationId: string | null): UiState {
  uiState.devtools.focusedAnnotationId = annotationId
  return getUiState()
}

export function setCommentOverlayVisible(visible: boolean): UiState {
  uiState.overlays.commentOverlayVisible = visible
  return getUiState()
}

export function setSelectionMarqueeVisible(visible: boolean): UiState {
  uiState.overlays.selectionMarqueeVisible = visible
  return getUiState()
}

export function setDevtoolsWidth(width: number): UiState {
  uiState.devtools.width = width
  return getUiState()
}

export function selectedEntityIds(ui: UiState = uiState): string[] {
  if (ui.selection.kind === 'single-entity') {
    return [ui.selection.entityId]
  }
  if (ui.selection.kind === 'multi-entity') {
    return ui.selection.entityIds
  }
  return []
}

export function selectedCanvasTargets(ui: UiState = uiState): CanvasSelectableTarget[] {
  if (ui.selection.kind === 'single-entity') {
    return [{ id: ui.selection.entityId, kind: ui.selection.entityKind }]
  }
  if (ui.selection.kind === 'multi-entity') {
    const { entityIds, entityKindsById } = ui.selection
    return entityIds
      .map((entityId) => ({
        id: entityId,
        kind: entityKindsById[entityId] ?? 'page',
      }))
  }
  return []
}

export function selectedEntityId(ui: UiState = uiState): string | null {
  if (ui.selection.kind === 'single-entity') return ui.selection.entityId
  if (ui.selection.kind === 'multi-entity') return ui.selection.entityIds[0] ?? null
  return null
}

export function selectedGroupId(ui: UiState = uiState): string | null {
  if (ui.selection.kind === 'single-entity' && ui.selection.entityKind === 'group') {
    return ui.selection.entityId
  }
  return null
}

export function activeTool(ui: UiState = uiState): Tool {
  return ui.activeTool
}

export function isCommentOverlayVisible(ui: UiState = uiState): boolean {
  return ui.overlays.commentOverlayVisible
}

export function isSelectionMarqueeVisible(ui: UiState = uiState): boolean {
  return ui.overlays.selectionMarqueeVisible
}

export function devtoolsOpen(ui: UiState = uiState): boolean {
  return ui.devtools.open
}

export function leftSidebarOpen(ui: UiState = uiState): boolean {
  return ui.leftSidebarOpen
}

export function toolbarDropdownOpen(ui: UiState = uiState): boolean {
  return ui.toolbarDropdownOpen
}

export function devtoolsPanelTab(ui: UiState = uiState): DevtoolsPanelTab {
  return ui.devtools.activeTab
}

export function focusedAnnotationId(ui: UiState = uiState): string | null {
  return ui.devtools.focusedAnnotationId
}

export function devtoolsWidth(ui: UiState = uiState): number {
  return ui.devtools.width
}

export function selectedPageIndex(
  pageIds: string[],
  ui: UiState = uiState,
): number | null {
  const activePageId =
    ui.selection.kind === 'multi-entity'
      ? ui.selection.entityIds.find((entityId) => pageIds.includes(entityId)) ?? null
      : selectedEntityId(ui)
  if (!activePageId) return null
  const index = pageIds.indexOf(activePageId)
  return index >= 0 ? index : null
}

function cloneUiState(input: UiState): UiState {
  return {
    selection:
      input.selection.kind === 'multi-entity'
        ? {
            kind: 'multi-entity',
            entityIds: [...input.selection.entityIds],
            entityKindsById: { ...input.selection.entityKindsById },
          }
        : { ...input.selection },
    activeTool: input.activeTool,
    leftSidebarOpen: input.leftSidebarOpen,
    toolbarDropdownOpen: input.toolbarDropdownOpen,
    devtools: { ...input.devtools },
    overlays: { ...input.overlays },
  }
}
