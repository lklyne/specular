import { useEffect, useState } from 'react'
import type {
  AgentPresenceCursor,
  DrawingBrushType,
  Tool,
  ToolbarSelectionData,
} from '../../shared/types'
import { toolbarApi } from './toolbarApi'

export const ZOOM_PRESETS = [10, 25, 50, 75, 100, 150, 200] as const

const EMPTY_SELECTION: ToolbarSelectionData = {
  activePageId: null,
  selectedEntityIds: [],
  selectionCount: 0,
  availablePageCount: 0,
  activeTabId: null,
  activeTabName: null,
  activeTool: { kind: 'select' },
  drawBrushType: 'pen',
  drawColor: '1',
  stickyColor: 'neutral',
  shapeColor: '1',
}

export interface ToolbarState {
  zoomPercent: number
  leftSidebarOpen: boolean
  devtoolsOpen: boolean
  activeTool: Tool
  drawBrushType: DrawingBrushType
  drawColor: string
  stickyColor: string
  shapeColor: string
  selection: ToolbarSelectionData
  currentPresetValue: (typeof ZOOM_PRESETS)[number] | null
  hasSelection: boolean
  agentCursors: AgentPresenceCursor[]
}

export function useToolbarState(): ToolbarState {
  const [zoomPercent, setZoomPercent] = useState(100)
  const [leftSidebarOpen, setLeftSidebarOpen] = useState(false)
  const [devtoolsOpen, setDevtoolsOpen] = useState(false)
  const [selection, setSelection] = useState<ToolbarSelectionData>(EMPTY_SELECTION)
  const [agentCursors, setAgentCursors] = useState<AgentPresenceCursor[]>([])

  useEffect(() => {
    const cleanupZoom = toolbarApi.onZoomChanged((value) => {
      setZoomPercent(value)
    })
    const cleanupSelection = toolbarApi.onSelectionChanged((data) => {
      setSelection(data)
    })
    const cleanupLeftSidebar = toolbarApi.onLeftSidebarChanged((open) => setLeftSidebarOpen(open))
    const cleanupDevtools = toolbarApi.onDevtoolsChanged((open) => setDevtoolsOpen(open))
    const cleanupPresence = toolbarApi.onAgentPresenceChanged((cursors) => {
      setAgentCursors(cursors)
    })

    return () => {
      cleanupZoom()
      cleanupSelection()
      cleanupLeftSidebar()
      cleanupDevtools()
      cleanupPresence()
    }
  }, [])

  const currentPresetValue = ZOOM_PRESETS.includes(zoomPercent as (typeof ZOOM_PRESETS)[number])
    ? (zoomPercent as (typeof ZOOM_PRESETS)[number])
    : null
  const hasSelection = selection.selectionCount > 0

  return {
    zoomPercent,
    leftSidebarOpen,
    devtoolsOpen,
    activeTool: selection.activeTool,
    drawBrushType: selection.drawBrushType,
    drawColor: selection.drawColor,
    stickyColor: selection.stickyColor,
    shapeColor: selection.shapeColor,
    selection,
    currentPresetValue,
    hasSelection,
    agentCursors,
  }
}
