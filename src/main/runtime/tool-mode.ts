// Tool mode management — single `activeTool` source of truth (ADR 0005).

import type { DevtoolsPanelTab, Tool } from '../../shared/types'
import { isAnnotationTool, isOneShot, isWorkingTool, toolAnnotateOverlay } from '../../shared/tool'
import { isFocusSessionActive, setFocusAnnotationsVisible } from './focus-session'
import { pages } from './runtime-context'
import { markDirty } from './layout-dirty'
import {
  activeTool as uiActiveTool,
  devtoolsPanelTab as uiDevtoolsPanelTab,
  selectedPageIndex as uiSelectedPageIndex,
  setActiveTool as setUiActiveTool,
  setDevtoolsPanelTab as setUiDevtoolsPanelTab,
} from '../ui-state'
import {
  setHoveredInspectTarget,
  syncInspectionState,
  notifyDevtoolsPanelData,
} from './inspect-session'
import { requestLayout } from './surface-layout'

function syncAnnotationState(): void {
  const payload = toolAnnotateOverlay(uiActiveTool())
  for (const page of pages) {
    page.pageView.webContents.send('set-annotate-mode', payload)
  }
}

function applyToolSideEffects(prev: Tool, next: Tool): void {
  const wasAnnotation = isAnnotationTool(prev)
  const isAnnotation = isAnnotationTool(next)
  const wasInspect = prev.kind === 'inspect'
  const isInspect = next.kind === 'inspect'

  if (wasInspect && !isInspect) {
    setHoveredInspectTarget(null)
  }

  if (wasAnnotation || isAnnotation) {
    markDirty('canvas')
    syncAnnotationState()
  }

  syncInspectionState()

  if (wasAnnotation !== isAnnotation || wasInspect !== isInspect) {
    notifyDevtoolsPanelData()
  }

  requestLayout()
}

export function setActiveTool(tool: Tool): Tool {
  // Draw, comment, and inspect activate on an empty canvas — they don't
  // require a page.
  const prev = uiActiveTool()
  if (toolsEqual(prev, tool)) {
    return prev
  }
  setUiActiveTool(tool)
  // A working tool latches annotation visibility on for the focus session, so a
  // sticky placed mid-focus stays visible after the one-shot tool reverts to
  // select. The user turns it back off via the focus bar's eye (ADR 0021).
  if (isWorkingTool(tool) && isFocusSessionActive()) {
    setFocusAnnotationsVisible(true)
    // The layout-update broadcast is gated on the 'canvas' dirty flag, and
    // applyToolSideEffects only dirties canvas for annotation tools — so a
    // non-annotation working tool (text/sticky/shape/page) would latch the eye
    // on without ever telling the renderer to lift the scrim.
    markDirty('canvas')
  }
  applyToolSideEffects(prev, tool)
  return uiActiveTool()
}

function toolsEqual(a: Tool, b: Tool): boolean {
  if (a.kind !== b.kind) return false
  if (a.kind === 'add-page' && b.kind === 'add-page') {
    return (
      a.presetIndex === b.presetIndex &&
      a.customSize === b.customSize &&
      a.sourcePageId === b.sourcePageId
    )
  }
  return true
}

export function clearActiveTool(): Tool {
  return setActiveTool({ kind: 'select' })
}

export function finishOneShotPlacement(): void {
  const tool = uiActiveTool()
  if (isOneShot(tool.kind)) {
    setActiveTool({ kind: 'select' })
  }
}

export function activeTool(): Tool {
  return uiActiveTool()
}

export function isAnnotateMode(): boolean {
  return uiActiveTool().kind === 'comment'
}

export function setDevtoolsPanelTab(tab: DevtoolsPanelTab): { needsDevtoolsAttach: boolean; attachPageIndex: number | null } {
  if (uiDevtoolsPanelTab() === tab) return { needsDevtoolsAttach: false, attachPageIndex: null }
  setUiDevtoolsPanelTab(tab)
  if (tab !== 'inspect') {
    setHoveredInspectTarget(null)
  }
  let attachPageIndex: number | null = null
  if (tab === 'browser-devtools') {
    const selectedPageIdx = uiSelectedPageIndex(pages.map((p) => p.id))
    if (selectedPageIdx !== null) {
      attachPageIndex = selectedPageIdx
    }
  }
  syncInspectionState()
  notifyDevtoolsPanelData()
  return { needsDevtoolsAttach: attachPageIndex !== null, attachPageIndex }
}
