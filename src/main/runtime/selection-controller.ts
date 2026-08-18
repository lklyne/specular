import type { CanvasEntityKind, UiState } from '../../shared/types'
import type { SelectionMutationMode } from '../../shared/selection-modifiers'
import {
  devtoolsPanelTab as uiDevtoolsPanelTab,
  getUiState,
  isCommentOverlayVisible,
  selectedEntityIds as uiSelectedEntityIds,
  setDevtoolsPanelTab as setUiDevtoolsPanelTab,
  setSelection as setUiSelection,
} from '../ui-state'
import {
  findPageById,
  hoverTarget,
  interactionState,
  interactivePageId,
  pages,
  setHoverTarget,
  setInteractivePageId,
} from './runtime-context'
import { workspaceEdges, workspaceGroups } from './space-model'
import { cancelActive as cancelActiveInteraction } from './interaction-controller'
import { clearInspectTargets, notifyDevtoolsPanelData, syncInspectionState } from './inspect-session'
import { requestLayout } from './viewport-control'
import { sendInteractiveState } from './overlay-manager'
import { savePreferences } from './preferences'
import { drawingEntities } from './drawing-entity-state'
import { fileEntities } from './file-entity-state'
import { shapeEntities } from './shape-entity-state'
import { textEntities } from './text-entity-state'
import {
  shouldFocusSelectedPage,
  type FocusSelectionInput,
} from '../../shared/should-focus-selected-page'
import { canvasInteractionModeKind } from '../../shared/gesture-utils'

type SelectionCommand =
  | { kind: 'none' }
  | { kind: 'single-entity'; entityId: string; entityKind: CanvasEntityKind }
  | {
      kind: 'multi-entity'
      entityIds: string[]
      entityKindsById: Partial<Record<string, CanvasEntityKind>>
    }

type CommitOptions = {
  clearHover?: boolean
  clearInteraction?: boolean
  clearInspect?: boolean
  notifyDevtools?: boolean
  syncInspection?: boolean
}

function predicateSelectionInput(selection: UiState['selection']): FocusSelectionInput {
  if (selection.kind === 'single-entity') {
    return {
      kind: 'single-entity',
      entityId: selection.entityId,
      entityKind: selection.entityKind,
    }
  }
  if (selection.kind === 'multi-entity') {
    return { kind: 'multi-entity', entityIds: selection.entityIds }
  }
  return { kind: 'none' }
}

/**
 * Predicate-derived "which page should hold keyboard + receive forwarded
 * input." Single source of truth for the focus reconciler, page cursor
 * bridge, and Escape handling. See `shouldFocusSelectedPage`.
 */
export function currentKeyboardTargetPageId(): string | null {
  const ui = getUiState()
  return shouldFocusSelectedPage({
    selection: predicateSelectionInput(ui.selection),
    interactionKind: canvasInteractionModeKind(interactionState),
    activeTool: ui.activeTool,
    commentOverlayActive: isCommentOverlayVisible(ui),
    interactivePageId: interactivePageId(),
  })
}

function selectionEquals(a: UiState['selection'], b: SelectionCommand): boolean {
  if (a.kind !== b.kind) return false
  if (a.kind === 'none' && b.kind === 'none') return true
  if (a.kind === 'single-entity' && b.kind === 'single-entity') {
    return a.entityId === b.entityId && a.entityKind === b.entityKind
  }
  if (a.kind === 'multi-entity' && b.kind === 'multi-entity') {
    if (a.entityIds.length !== b.entityIds.length) return false
    return a.entityIds.every((id, index) => {
      if (b.entityIds[index] !== id) return false
      return (a.entityKindsById[id] ?? 'page') === (b.entityKindsById[id] ?? 'page')
    })
  }
  return false
}

export function resolveEntityKind(entityId: string): CanvasEntityKind {
  if (findPageById(entityId)) return 'page'
  if (textEntities.some((entity) => entity.id === entityId)) return 'text'
  if (fileEntities.some((entity) => entity.id === entityId)) return 'file'
  if (drawingEntities.some((entity) => entity.id === entityId)) return 'drawing'
  if (shapeEntities.some((entity) => entity.id === entityId)) return 'shape'
  if (workspaceGroups.some((group) => group.id === entityId)) return 'group'
  if (workspaceEdges.some((edge) => edge.id === entityId)) return 'edge'
  return 'page'
}

function browserDevtoolsSelectionAllowed(nextSelection: SelectionCommand): boolean {
  return nextSelection.kind === 'single-entity' && nextSelection.entityKind === 'page'
}

function normalizeEntitySelection(entityIds: string[]): SelectionCommand {
  const nextEntityIds = [...new Set(entityIds)].filter((entityId) => {
    if (findPageById(entityId)) return true
    if (textEntities.some((entity) => entity.id === entityId)) return true
    if (fileEntities.some((entity) => entity.id === entityId)) return true
    if (drawingEntities.some((entity) => entity.id === entityId)) return true
    if (shapeEntities.some((entity) => entity.id === entityId)) return true
    if (workspaceGroups.some((group) => group.id === entityId)) return true
    return workspaceEdges.some((edge) => edge.id === entityId)
  })

  if (!nextEntityIds.length) return { kind: 'none' }
  if (nextEntityIds.length === 1) {
    const entityId = nextEntityIds[0]
    return { kind: 'single-entity', entityId, entityKind: resolveEntityKind(entityId) }
  }

  return {
    kind: 'multi-entity',
    entityIds: nextEntityIds,
    entityKindsById: Object.fromEntries(
      nextEntityIds.map((entityId) => [entityId, resolveEntityKind(entityId)]),
    ) as Partial<Record<string, CanvasEntityKind>>,
  }
}

function commitSelection(
  nextSelection: SelectionCommand,
  options?: CommitOptions,
): boolean {
  const shouldClearHover = options?.clearHover ?? nextSelection.kind === 'none'
  const shouldClearInteraction = options?.clearInteraction ?? false
  const shouldClearInspect = options?.clearInspect ?? false
  const shouldSyncInspection = options?.syncInspection ?? true
  const shouldNotifyDevtools = options?.notifyDevtools ?? true

  if (selectionEquals(getUiState().selection, nextSelection)) {
    if (shouldClearHover && hoverTarget) {
      setHoverTarget(null)
    }
    if (shouldClearInteraction) cancelActiveInteraction('external')
    if (shouldClearInspect) clearInspectTargets()
    sendInteractiveState()
    if (shouldSyncInspection) syncInspectionState()
    if (shouldNotifyDevtools) notifyDevtoolsPanelData()
    requestLayout()
    return false
  }

  setUiSelection(nextSelection)

  // Select-first / interact-second (#124): moving the selection off the entered
  // page drops it back to selected-only — blocker back on, keyboard back to the
  // canvas. Re-selecting the same page (selectionEquals above) keeps it entered.
  const enteredId = interactivePageId()
  if (
    enteredId !== null &&
    !(
      nextSelection.kind === 'single-entity' &&
      nextSelection.entityKind === 'page' &&
      nextSelection.entityId === enteredId
    )
  ) {
    setInteractivePageId(null)
  }

  if (!browserDevtoolsSelectionAllowed(nextSelection) && uiDevtoolsPanelTab() === 'browser-devtools') {
    setUiDevtoolsPanelTab('comments')
    savePreferences()
  }

  if (shouldClearHover) {
    setHoverTarget(null)
  }
  if (shouldClearInteraction) cancelActiveInteraction('external')
  if (shouldClearInspect) clearInspectTargets()

  sendInteractiveState()
  if (shouldSyncInspection) syncInspectionState()
  if (shouldNotifyDevtools) notifyDevtoolsPanelData()
  requestLayout()
  return true
}

export function selectNone(options?: CommitOptions): boolean {
  return commitSelection({ kind: 'none' }, { clearHover: true, ...options })
}

export function selectPageById(
  pageId: string,
  options?: CommitOptions,
): boolean {
  const page = findPageById(pageId)
  if (!page) return false
  return commitSelection(
    { kind: 'single-entity', entityId: page.id, entityKind: 'page' },
    options,
  )
}

export function selectPageByIndex(
  index: number,
  options?: CommitOptions,
): boolean {
  if (index < 0 || index >= pages.length) return false
  return selectPageById(pages[index].id, options)
}

export function selectEntity(
  entityId: string,
  entityKind: CanvasEntityKind,
  options?: CommitOptions,
): boolean {
  if (entityKind === 'page') {
    return selectPageById(entityId, options)
  }
  return commitSelection(
    { kind: 'single-entity', entityId, entityKind },
    { clearInteraction: true, ...options },
  )
}

export function selectEntities(
  entityIds: string[],
  options?: CommitOptions,
): boolean {
  const nextSelection = normalizeEntitySelection(entityIds)
  if (nextSelection.kind === 'none') return selectNone(options)
  if (nextSelection.kind === 'single-entity') {
    return selectEntity(nextSelection.entityId, nextSelection.entityKind, options)
  }
  return commitSelection(nextSelection, { clearInspect: true, ...options })
}

export function selectPages(
  pageIds: string[],
  options?: CommitOptions,
): boolean {
  const nextPageIds = [...new Set(pageIds)].filter((pageId) => Boolean(findPageById(pageId)))
  if (!nextPageIds.length) return selectNone(options)
  if (nextPageIds.length === 1) return selectPageById(nextPageIds[0], options)
  return commitSelection(
    {
      kind: 'multi-entity',
      entityIds: nextPageIds,
      entityKindsById: Object.fromEntries(nextPageIds.map((id) => [id, 'page' as const])),
    },
    { clearInspect: true, ...options },
  )
}

export function selectGroup(
  groupId: string,
  options?: CommitOptions,
): boolean {
  const group = workspaceGroups.find((candidate) => candidate.id === groupId)
  if (!group) return false
  return commitSelection(
    { kind: 'single-entity', entityId: groupId, entityKind: 'group' },
    { clearInteraction: true, ...options },
  )
}

export function enterGroup(
  groupId: string,
  options?: CommitOptions,
): boolean {
  const childIds = [
    ...pages.filter((page) => page.parentGroupId === groupId).map((page) => page.id),
    ...textEntities.filter((entity) => entity.parentGroupId === groupId).map((entity) => entity.id),
    ...fileEntities.filter((entity) => entity.parentGroupId === groupId).map((entity) => entity.id),
    ...shapeEntities.filter((entity) => entity.parentGroupId === groupId).map((entity) => entity.id),
    ...workspaceGroups.filter((group) => group.parentGroupId === groupId).map((group) => group.id),
  ]
  if (!childIds.length) return false
  return selectEntities(childIds, { clearInteraction: true, ...options })
}

export function applyEntitySelectionMutation(
  targetIds: string[],
  mode: SelectionMutationMode,
  options?: CommitOptions,
): boolean {
  if (mode === 'replace') return selectEntities(targetIds, options)

  const current = uiSelectedEntityIds()
  const currentSet = new Set(current)
  const targetSet = new Set(targetIds)

  let nextIds: string[]
  if (mode === 'add') {
    nextIds = [...new Set([...current, ...targetIds])]
  } else if (mode === 'remove') {
    nextIds = current.filter((id) => !targetSet.has(id))
  } else {
    // toggle: XOR — entities already selected are dropped, new ones are added
    const kept = current.filter((id) => !targetSet.has(id))
    const added = targetIds.filter((id) => !currentSet.has(id))
    nextIds = [...kept, ...added]
  }

  return selectEntities(nextIds, options)
}
