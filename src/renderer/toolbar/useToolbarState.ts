import type { Dispatch, RefObject, SetStateAction } from 'react'
import { useEffect, useRef, useState } from 'react'
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
  displayUrl: '',
  placeholder: '',
  canGoBack: false,
  canGoForward: false,
  isLoadingActivePage: false,
  loadingPageCount: 0,
  isLoadingAnySelected: false,
  loadingPhase: 'idle',
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
  addressValue: string
  setAddressValue: Dispatch<SetStateAction<string>>
  addressBarRef: RefObject<HTMLInputElement | null>
  currentPresetValue: (typeof ZOOM_PRESETS)[number] | null
  hasSelection: boolean
  agentCursors: AgentPresenceCursor[]
}

export function useToolbarState(): ToolbarState {
  const [zoomPercent, setZoomPercent] = useState(100)
  const [leftSidebarOpen, setLeftSidebarOpen] = useState(true)
  const [devtoolsOpen, setDevtoolsOpen] = useState(false)
  const [selection, setSelection] = useState<ToolbarSelectionData>(EMPTY_SELECTION)
  const [addressValue, setAddressValue] = useState('')
  const [agentCursors, setAgentCursors] = useState<AgentPresenceCursor[]>([])
  const addressBarRef = useRef<HTMLInputElement | null>(null)

  useEffect(() => {
    const cleanupZoom = toolbarApi.onZoomChanged((value) => {
      setZoomPercent(value)
    })
    const cleanupSelection = toolbarApi.onSelectionChanged((data) => {
      setSelection(data)
      setAddressValue(data.displayUrl)
    })
    const cleanupLeftSidebar = toolbarApi.onLeftSidebarChanged((open) => setLeftSidebarOpen(open))
    const cleanupDevtools = toolbarApi.onDevtoolsChanged((open) => setDevtoolsOpen(open))
    const cleanupPresence = toolbarApi.onAgentPresenceChanged((cursors) => {
      setAgentCursors(cursors)
    })
    let focusTimer: ReturnType<typeof setTimeout> | undefined
    const cleanupFocusAddress = toolbarApi.onFocusAddressBar(() => {
      clearTimeout(focusTimer)
      focusTimer = setTimeout(() => {
        addressBarRef.current?.focus()
        addressBarRef.current?.select()
      }, 50)
    })

    return () => {
      cleanupZoom()
      cleanupSelection()
      cleanupLeftSidebar()
      cleanupDevtools()
      cleanupPresence()
      cleanupFocusAddress()
      clearTimeout(focusTimer)
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
    addressValue,
    setAddressValue,
    addressBarRef,
    currentPresetValue,
    hasSelection,
    agentCursors,
  }
}
