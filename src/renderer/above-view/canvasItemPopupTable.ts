import type { ComponentType } from 'react'
import type {
  CanvasSceneEntity,
  CanvasSceneGroupEntity,
  LayoutUpdateData,
} from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { ToolKind } from '../../shared/tool'
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
import type { AnnotateHandler } from './annotationMath'

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
  filePopupReady: boolean
  /** Opens the region composer pre-anchored to a selection's union bounds
   *  (see useAnnotationDraftState.beginSelectionAnnotation). Every popup's
   *  Annotate button forwards to the same renderer-local handoff. */
  beginSelectionAnnotation: AnnotateHandler
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
  /** PagePopup and FilePopup double as the focus bar for their target kind —
   *  exempt the one that owns the running session from the tool mutex so the
   *  focus controls stay visible (ADR 0021). The other stays under the mutex,
   *  so only ever one bar is exempt. */
  focusExempt?: (ctx: PopupContext) => boolean
}

function selectionRow<P extends object>(
  key: string,
  Component: ComponentType<P>,
  mapProps: (ctx: PopupContext) => P,
  focusExempt?: (ctx: PopupContext) => boolean,
): SelectionPopupRow {
  return {
    key,
    Component: Component as ComponentType<any>,
    mapProps: mapProps as (ctx: PopupContext) => Record<string, unknown>,
    focusExempt,
  }
}

const focusTargetKindIs =
  (kind: 'page' | 'file') =>
  (ctx: PopupContext): boolean =>
    ctx.layout.focusPresentation?.target.kind === kind

// Selection-mode popups (ADR 0008 §4): all mount together under the mutex; each
// early-returns to nothing when its slice is empty, so the row list is static.
export const SELECTION_POPUPS: SelectionPopupRow[] = [
  selectionRow('text', StickyNotePopover, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedTextEntities: sameKindEntities(ctx.sameKindSelection, 'text'),
    popupReady: ctx.textPopupReady,
    onAnnotate: ctx.beginSelectionAnnotation,
  })),
  selectionRow('group', GroupPopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedGroup: ctx.selectedGroup,
    interactionIdle: ctx.interactionIdle,
    onAnnotate: ctx.beginSelectionAnnotation,
  })),
  selectionRow('shape', ShapePopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedShapes: sameKindEntities(ctx.sameKindSelection, 'shape'),
    interactionIdle: ctx.interactionIdle,
    onAnnotate: ctx.beginSelectionAnnotation,
  })),
  selectionRow('drawing', DrawingPopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    selectedDrawings: sameKindEntities(ctx.sameKindSelection, 'drawing'),
    interactionIdle: ctx.interactionIdle,
    onAnnotate: ctx.beginSelectionAnnotation,
  })),
  selectionRow(
    'file',
    FilePopup,
    (ctx) => ({
      api: ctx.api,
      isDark: ctx.isDark,
      layout: ctx.layout,
      selectedFiles: sameKindEntities(ctx.sameKindSelection, 'file'),
      popupReady: ctx.filePopupReady,
      onAnnotate: ctx.beginSelectionAnnotation,
    }),
    focusTargetKindIs('file'),
  ),
  // Mixed-kind fallback: renders only when the selection spans kinds, so it
  // never doubles up with a per-kind popup.
  selectionRow('multi', MultiSelectPopup, (ctx) => ({
    api: ctx.api,
    isDark: ctx.isDark,
    layout: ctx.layout,
    mixed: ctx.sameKindSelection === null,
    onAnnotate: ctx.beginSelectionAnnotation,
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
      onAnnotate: ctx.beginSelectionAnnotation,
    }),
    focusTargetKindIs('page'),
  ),
]
