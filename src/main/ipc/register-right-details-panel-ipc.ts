import { BrowserWindow, dialog, ipcMain } from 'electron'
import type { AnnotationCreateRequest, EdgeEnd, EdgeSide } from '../../shared/types'
import { setFixConfig } from '../runtime/preferences'
import {
  bindOriginToRepoPath,
  removeBindingByOrigin,
  setBindingAutoFix,
} from '../runtime/dev-server-manager'
import {
  fixAnnotation,
  fixPendingAnnotationsForOrigin,
} from '../agent-fix/fix-orchestrator'
import { notifyDevtoolsPanelData } from '../runtime/inspect-session'
import {
  deleteEdge,
  updateEdge,
  setPagePreset,
  setPageCustom,
  setDeviceOrientation,
  toggleDeviceShell,
  toggleSvgDeviceShell,
  setFilePreset,
  setFileCustom,
  setFileDeviceOrientation,
  toggleFileDeviceShell,
} from '../runtime/document-commands'
import { togglePageLinked } from '../navigation-sync'
import { deletePages } from '../workspace-entities'
import { duplicatePageFromSource } from '../workspace-pages'
import {
  dismissBrowserDevTools,
  openDevToolsForSelectedPage,
  openInspectPanel,
  selectPageById,
  setInspectNodeFromPanel,
  setSelectedInspectNodeById,
  setSelectedInspectTarget,
} from '../runtime/ui-actions'
import { requestLayout } from '../runtime/viewport-control'
import { markDirty } from '../runtime/layout-dirty'
import {
  addAnnotationReply,
  createAnnotation,
  deleteAnnotation,
  updateAnnotationStatus,
} from '../workspace-annotations'
import { pages } from '../runtime/page-runtime'
import {
  forwardOverrideToPage,
  type ComponentPropOverridePayload,
  type ComponentTokenOverridePayload,
} from './component-override'

type SingleFieldCommand = {
  /** Payload key carrying the target id. */
  key: 'annotationId' | 'origin' | 'pageId' | 'fileId'
  /** Trim the id and require it non-empty before running. */
  trim?: boolean
  /** Extra payload validation beyond the id field. */
  accept?: (payload: Record<string, unknown>) => boolean
  run: (id: string, payload: Record<string, unknown>) => void
}

const hasPresetIndex = (payload: Record<string, unknown>): boolean =>
  typeof payload.presetIndex === 'number'

const hasOrientation = (payload: Record<string, unknown>): boolean =>
  payload.orientation === 'portrait' || payload.orientation === 'landscape'

/** Channels that validate one id field, then call one function. */
const SINGLE_FIELD_COMMANDS: Record<string, SingleFieldCommand> = {
  // --- Annotations and fixes ---
  'right-details-panel-resolve-annotation': {
    key: 'annotationId',
    trim: true,
    run: (id) => updateAnnotationStatus(id, 'resolved'),
  },
  'right-details-panel-delete-annotation': {
    key: 'annotationId',
    trim: true,
    run: (id) => deleteAnnotation(id),
  },
  'right-details-panel-trigger-fix-comments': {
    key: 'origin',
    trim: true,
    run: (origin) => fixPendingAnnotationsForOrigin(origin),
  },
  'right-details-panel-fix-single-annotation': {
    key: 'annotationId',
    trim: true,
    run: (id) => fixAnnotation(id),
  },
  // --- Device Page ---
  'right-details-panel-set-page-preset': {
    key: 'pageId',
    accept: hasPresetIndex,
    run: (id, payload) => setPagePreset(id, payload.presetIndex as number),
  },
  'right-details-panel-set-page-custom': {
    key: 'pageId',
    run: (id) => setPageCustom(id),
  },
  'right-details-panel-set-device-orientation': {
    key: 'pageId',
    accept: hasOrientation,
    run: (id, payload) =>
      setDeviceOrientation(id, payload.orientation as 'portrait' | 'landscape'),
  },
  'right-details-panel-toggle-device-shell': {
    key: 'pageId',
    run: (id) => toggleDeviceShell(id),
  },
  'right-details-panel-toggle-svg-device-shell': {
    key: 'pageId',
    run: (id) => toggleSvgDeviceShell(id),
  },
  // --- File Device Settings ---
  'right-details-panel-set-file-preset': {
    key: 'fileId',
    accept: hasPresetIndex,
    run: (id, payload) => setFilePreset(id, payload.presetIndex as number),
  },
  'right-details-panel-set-file-custom': {
    key: 'fileId',
    run: (id) => setFileCustom(id),
  },
  'right-details-panel-set-file-device-orientation': {
    key: 'fileId',
    accept: hasOrientation,
    run: (id, payload) =>
      setFileDeviceOrientation(id, payload.orientation as 'portrait' | 'landscape'),
  },
  'right-details-panel-toggle-file-device-shell': {
    key: 'fileId',
    run: (id) => toggleFileDeviceShell(id),
  },
}

function registerSingleFieldHandlers(): void {
  for (const [channel, command] of Object.entries(SINGLE_FIELD_COMMANDS)) {
    ipcMain.on(channel, (_event, payload: Record<string, unknown> | undefined) => {
      if (!payload) return
      const raw = payload[command.key]
      if (typeof raw !== 'string') return
      const id = command.trim ? raw.trim() : raw
      if (!id) return
      if (command.accept && !command.accept(payload)) return
      command.run(id, payload)
    })
  }
}

export function registerRightDetailsPanelIpc(): void {
  ipcMain.on('right-details-panel-open-browser-devtools', () => {
    openDevToolsForSelectedPage()
  })

  ipcMain.on('right-details-panel-dismiss-browser-devtools', () => {
    dismissBrowserDevTools()
  })

  ipcMain.on('right-details-panel-clear-inspect-selection', () => {
    setSelectedInspectTarget(null)
    markDirty('canvas')
    requestLayout()
  })

  ipcMain.on(
    'right-details-panel-select-page',
    (_event, payload: { pageId?: string } | undefined) => {
      const pageId = payload?.pageId?.trim()
      if (!pageId) return
      selectPageById(pageId)
    },
  )

  ipcMain.on(
    'right-details-panel-hover-node',
    (_event, { pageId, nodeId }: { pageId: string; nodeId: string | null }) => {
      if (!pageId) return
      setInspectNodeFromPanel(pageId, nodeId, false)
      markDirty('canvas')
      requestLayout()
    },
  )

  ipcMain.on(
    'right-details-panel-select-node',
    (_event, { pageId, nodeId }: { pageId: string; nodeId: string | null }) => {
      if (!pageId) return
      if (selectPageById(pageId)) {
        openInspectPanel()
      }
      setSelectedInspectNodeById(pageId, nodeId)
      setInspectNodeFromPanel(pageId, nodeId, true)
      markDirty('canvas')
      requestLayout()
    },
  )

  ipcMain.on(
    'right-details-panel-edit-component-prop',
    (
      _event,
      { pageId, componentId, propPath, value }: ComponentPropOverridePayload,
    ) => {
      forwardOverrideToPage(pageId, 'override-props', {
        componentId,
        propPath,
        value,
      })
    },
  )

  ipcMain.on(
    'right-details-panel-edit-component-token',
    (
      _event,
      { pageId, componentId, token, value, selector }: ComponentTokenOverridePayload,
    ) => {
      forwardOverrideToPage(pageId, 'override-token', {
        componentId,
        token,
        value,
        selector,
      })
    },
  )

  ipcMain.on(
    'right-details-panel-create-annotation',
    (_event, request: AnnotationCreateRequest) => {
      createAnnotation(request)
    },
  )

  ipcMain.on(
    'right-details-panel-reply-annotation',
    (_event, payload: { annotationId?: string; text?: string } | undefined) => {
      const annotationId = payload?.annotationId?.trim()
      const text = payload?.text?.trim()
      if (!annotationId || !text) return
      addAnnotationReply(annotationId, 'user', text)
    },
  )

  registerSingleFieldHandlers()

  ipcMain.on(
    'right-details-panel-set-auto-fix',
    (_event, payload: { origin?: string; enabled?: boolean } | undefined) => {
      const origin = payload?.origin?.trim()
      if (!origin) return
      const enabled = !!payload?.enabled
      const mutated = setBindingAutoFix(origin, enabled)
      if (!mutated) return
      notifyDevtoolsPanelData()
      if (enabled) {
        fixPendingAnnotationsForOrigin(origin)
      }
    },
  )

  ipcMain.on(
    'right-details-panel-pick-repo-for-origin',
    async (event, payload: { origin?: string } | undefined) => {
      const origin = payload?.origin?.trim()
      if (!origin) return
      const win = BrowserWindow.fromWebContents(event.sender) ?? BrowserWindow.getFocusedWindow()
      const dialogOpts: Electron.OpenDialogOptions = { title: `Choose repo for ${origin}`, properties: ['openDirectory'] }
      const result = win
        ? await dialog.showOpenDialog(win, dialogOpts)
        : await dialog.showOpenDialog(dialogOpts)
      if (result.canceled || result.filePaths.length === 0) return
      bindOriginToRepoPath(origin, result.filePaths[0])
      notifyDevtoolsPanelData()
    },
  )

  ipcMain.on(
    'right-details-panel-remove-origin-binding',
    (_event, payload: { origin?: string } | undefined) => {
      const origin = payload?.origin?.trim()
      if (!origin) return
      if (removeBindingByOrigin(origin)) notifyDevtoolsPanelData()
    },
  )

  ipcMain.on(
    'right-details-panel-set-fix-config',
    (_event, payload: { model?: string; permissions?: string } | undefined) => {
      if (!payload) return
      setFixConfig(payload as { model?: 'opus' | 'sonnet' | 'haiku'; permissions?: 'dangerously' | 'default' })
      notifyDevtoolsPanelData()
    },
  )

  ipcMain.on(
    'right-details-panel-update-edge',
    (_event, payload: { id: string; patch: { fromEnd?: EdgeEnd; toEnd?: EdgeEnd; fromSide?: EdgeSide; toSide?: EdgeSide; color?: string; label?: string } }) => {
      if (!payload?.id) return
      updateEdge(payload.id, payload.patch)
    },
  )

  ipcMain.on(
    'right-details-panel-delete-edge',
    (_event, payload: { id: string }) => {
      if (!payload?.id) return
      deleteEdge(payload.id)
    },
  )

  // --- Page actions ---

  ipcMain.on(
    'right-details-panel-duplicate-page',
    (_event, payload: { pageId: string }) => {
      if (!payload?.pageId) return
      if (!pages.some((p) => p.id === payload.pageId)) return
      duplicatePageFromSource({ sourcePageId: payload.pageId })
    },
  )

  ipcMain.on(
    'right-details-panel-toggle-linked-page',
    (_event, payload: { pageId: string }) => {
      const page = pages.find((p) => p.id === payload?.pageId)
      if (!page) return
      togglePageLinked(page)
    },
  )

  ipcMain.on(
    'right-details-panel-delete-page',
    (_event, payload: { pageId: string }) => {
      if (!payload?.pageId) return
      if (!pages.some((p) => p.id === payload.pageId)) return
      deletePages({ pageIds: [payload.pageId] })
    },
  )
}
