import type { ComponentType } from 'react'
import type {
  CanvasSceneEntity,
  CanvasSceneGroupEntity,
  LayoutUpdateData,
} from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { ToolKind } from '../../shared/tool'
import type { FileJsonModeMap } from './FileBodyLayer'
import { PagePopup } from './PagePopup'
import { FilePopup } from './FilePopup'
import { DrawingPopup } from './DrawingPopup'
import { DrawToolPopup } from './DrawToolPopup'
import { GroupPopup } from './GroupPopup'
import { MultiSelectPopup } from './MultiSelectPopup'
import { PageToolPopup } from './PageToolPopup'
import { ShapePopup } from './ShapePopup'
import { ShapeToolPopup } from './ShapeToolPopup'
import { StickyNotePopover } from './StickyNotePopover'
import { TextToolPopup } from './TextToolPopup'

/**
 * Same-kind multi-select (ADR 0008 §4): the selected entities plus their shared
 * kind, iff every selected id resolves to one uniform kind; otherwise null.
 * Computed once so each selection popup slices its own kind out of it.
 */
export type SameKindSelection = {
  kind: CanvasSceneEntity['kind']
  entities: CanvasSceneEntity[]
} | null

export function computeSameKindSelection(layout: LayoutUpdateData): SameKindSelection {
  const ids = layout.selectedEntityIds
  if (ids.length === 0) return null
  let kind: CanvasSceneEntity['kind'] | null = null
  const entities: CanvasSceneEntity[] = []
  for (const id of ids) {
    const entity = layout.entities.find((e) => e.id === id)
    if (!entity) return null
    if (kind === null) kind = entity.kind
    else if (entity.kind !== kind) return null
    entities.push(entity)
  }
  return kind === null ? null : { kind, entities }
}

export function sameKindEntities<K extends CanvasSceneEntity['kind']>(
  selection: SameKindSelection,
  kind: K,
): Extract<CanvasSceneEntity, { kind: K }>[] {
  return selection && selection.kind === kind
    ? (selection.entities as Extract<CanvasSceneEntity, { kind: K }>[])
    : []
}

/** Shared inputs every popup row reads; per-row mapProps picks what it needs. */
export type PopupContext = {
  api: CanvasBgElectronAPI
  isDark: boolean
  layout: LayoutUpdateData
  interactionIdle: boolean
  sameKindSelection: SameKindSelection
  selectedGroup: CanvasSceneGroupEntity | null
  textPopupReady: boolean
  fileJsonModeMap: FileJsonModeMap
  setFileJsonMode: (entityId: string, jsonMode: boolean) => void
}

export type ToolPopupRow = {
  toolKind: ToolKind
  Component: ComponentType<any>
  extraProps?: Record<string, unknown>
}

// Tool-mode popups (ADR 0008 §1): keyed by the active tool; exactly one mounts,
// and it always wins the tool-vs-selection mutex (§2).
export const TOOL_POPUPS: ToolPopupRow[] = [
  { toolKind: 'add-page', Component: PageToolPopup },
  { toolKind: 'add-text', Component: TextToolPopup, extraProps: { style: 'plain' } },
  { toolKind: 'add-sticky', Component: TextToolPopup, extraProps: { style: 'sticky' } },
  { toolKind: 'add-shape', Component: ShapeToolPopup },
  { toolKind: 'draw', Component: DrawToolPopup },
]

export type SelectionPopupRow = {
  key: string
  Component: ComponentType<any>
  mapProps: (ctx: PopupContext) => Record<string, unknown>
  /** PagePopup doubles as the focus bar — exempt it from the tool mutex while a
   *  focus session is active so the focus controls stay visible (ADR 0021). */
  focusExempt?: boolean
}

function selectionRow<P extends object>(
  key: string,
  Component: ComponentType<P>,
  mapProps: (ctx: PopupContext) => P,
  focusExempt = false,
): SelectionPopupRow {
  return {
    key,
    Component: Component as ComponentType<any>,
    mapProps: mapProps as (ctx: PopupContext) => Record<string, unknown>,
    focusExempt,
  }
}

// Selection-mode popups (ADR 0008 §4): all mount together under the mutex; each
// early-returns to nothing when its slice is empty, so the row list is static.
export const SELECTION_POPUPS: SelectionPopupRow[] = [
  selectionRow('text', StickyNotePopover, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedTextEntities: sameKindEntities(ctx.sameKindSelection, 'text'),
    popupReady: ctx.textPopupReady,
  })),
  selectionRow('group', GroupPopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedGroup: ctx.selectedGroup,
    interactionIdle: ctx.interactionIdle,
  })),
  selectionRow('shape', ShapePopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedShapes: sameKindEntities(ctx.sameKindSelection, 'shape'),
    interactionIdle: ctx.interactionIdle,
  })),
  selectionRow('drawing', DrawingPopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedDrawings: sameKindEntities(ctx.sameKindSelection, 'drawing'),
    interactionIdle: ctx.interactionIdle,
  })),
  selectionRow('file', FilePopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedFiles: sameKindEntities(ctx.sameKindSelection, 'file'),
    interactionIdle: ctx.interactionIdle,
    fileJsonModeMap: ctx.fileJsonModeMap,
    setFileJsonMode: ctx.setFileJsonMode,
  })),
  // Mixed-kind fallback: renders only when the selection spans kinds, so it
  // never doubles up with a per-kind popup.
  selectionRow('multi', MultiSelectPopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    mixed: ctx.sameKindSelection === null,
  })),
  selectionRow(
    'page',
    PagePopup,
    (ctx) => ({
      api: ctx.api,
      isDark: ctx.isDark,
      layout: ctx.layout,
      selectedPages: sameKindEntities(ctx.sameKindSelection, 'page'),
      interactionIdle: ctx.interactionIdle,
    }),
    true,
  ),
]
