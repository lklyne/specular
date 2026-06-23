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
import { navigatePage, togglePageLinked } from '../navigation-sync'
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
import { requestLayout } from '../runtime/surface-layout'
import { markDirty } from '../runtime/layout-dirty'
import {
  addAnnotationReply,
  createAnnotation,
  deleteAnnotation,
  updateAnnotationStatus,
} from '../workspace-annotations'
import { pages } from '../runtime/page-runtime'

type ComponentPropOverridePayload = {
  pageId: string
  componentId: string
  propPath: string[]
  value: unknown
}

type ComponentTokenOverridePayload = {
  pageId: string
  componentId?: string
  token: string
  value: string
  selector?: string
}

function forwardOverrideToPage(
  pageId: string,
  channel: 'override-props' | 'override-token',
  payload: Record<string, unknown>,
): void {
  const page = pages.find((candidate) => candidate.id === pageId)
  if (!page || page.pageView.webContents.isDestroyed()) return
  page.pageView.webContents.send(channel, payload)
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

  ipcMain.on(
    'right-details-panel-resolve-annotation',
    (_event, payload: { annotationId?: string } | undefined) => {
      const annotationId = payload?.annotationId?.trim()
      if (!annotationId) return
      updateAnnotationStatus(annotationId, 'resolved')
    },
  )

  ipcMain.on(
    'right-details-panel-delete-annotation',
    (_event, payload: { annotationId?: string } | undefined) => {
      const annotationId = payload?.annotationId?.trim()
      if (!annotationId) return
      deleteAnnotation(annotationId)
    },
  )

  ipcMain.on(
    'right-details-panel-trigger-fix-comments',
    (_event, payload: { origin?: string } | undefined) => {
      const origin = payload?.origin?.trim()
      if (!origin) return
      fixPendingAnnotationsForOrigin(origin)
    },
  )

  ipcMain.on(
    'right-details-panel-fix-single-annotation',
    (_event, payload: { annotationId?: string } | undefined) => {
      const annotationId = payload?.annotationId?.trim()
      if (!annotationId) return
      fixAnnotation(annotationId)
    },
  )

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

  // --- Device Page ---

  ipcMain.on(
    'right-details-panel-set-page-preset',
    (_event, payload: { pageId: string; presetIndex: number }) => {
      if (!payload?.pageId || typeof payload.presetIndex !== 'number') return
      setPagePreset(payload.pageId, payload.presetIndex)
    },
  )

  ipcMain.on(
    'right-details-panel-set-page-custom',
    (_event, payload: { pageId: string }) => {
      if (!payload?.pageId) return
      setPageCustom(payload.pageId)
    },
  )

  ipcMain.on(
    'right-details-panel-set-device-orientation',
    (_event, payload: { pageId: string; orientation: string }) => {
      if (!payload?.pageId) return
      if (payload.orientation !== 'portrait' && payload.orientation !== 'landscape') return
      setDeviceOrientation(payload.pageId, payload.orientation)
    },
  )

  ipcMain.on(
    'right-details-panel-toggle-device-shell',
    (_event, payload: { pageId: string }) => {
      if (!payload?.pageId) return
      toggleDeviceShell(payload.pageId)
    },
  )

  ipcMain.on(
    'right-details-panel-toggle-svg-device-shell',
    (_event, payload: { pageId: string }) => {
      if (!payload?.pageId) return
      toggleSvgDeviceShell(payload.pageId)
    },
  )

  // --- File Device Settings ---

  ipcMain.on(
    'right-details-panel-set-file-preset',
    (_event, payload: { fileId: string; presetIndex: number }) => {
      if (!payload?.fileId || typeof payload.presetIndex !== 'number') return
      setFilePreset(payload.fileId, payload.presetIndex)
    },
  )

  ipcMain.on(
    'right-details-panel-set-file-custom',
    (_event, payload: { fileId: string }) => {
      if (!payload?.fileId) return
      setFileCustom(payload.fileId)
    },
  )

  ipcMain.on(
    'right-details-panel-set-file-device-orientation',
    (_event, payload: { fileId: string; orientation: string }) => {
      if (!payload?.fileId) return
      if (payload.orientation !== 'portrait' && payload.orientation !== 'landscape') return
      setFileDeviceOrientation(payload.fileId, payload.orientation)
    },
  )

  ipcMain.on(
    'right-details-panel-toggle-file-device-shell',
    (_event, payload: { fileId: string }) => {
      if (!payload?.fileId) return
      toggleFileDeviceShell(payload.fileId)
    },
  )

  // --- Page navigation & actions ---

  ipcMain.on(
    'right-details-panel-navigate-page',
    (_event, payload: { pageId: string; url: string }) => {
      const page = pages.find((p) => p.id === payload?.pageId)
      if (!page) return
      navigatePage(page, { type: 'load-url', url: payload.url })
    },
  )

  ipcMain.on(
    'right-details-panel-go-back-page',
    (_event, payload: { pageId: string }) => {
      const page = pages.find((p) => p.id === payload?.pageId)
      if (!page) return
      navigatePage(page, { type: 'go-back', fallbackUrl: page.pageView.webContents.getURL() })
    },
  )

  ipcMain.on(
    'right-details-panel-go-forward-page',
    (_event, payload: { pageId: string }) => {
      const page = pages.find((p) => p.id === payload?.pageId)
      if (!page) return
      navigatePage(page, { type: 'go-forward', fallbackUrl: page.pageView.webContents.getURL() })
    },
  )

  ipcMain.on(
    'right-details-panel-reload-page',
    (_event, payload: { pageId: string }) => {
      const page = pages.find((p) => p.id === payload?.pageId)
      if (!page) return
      navigatePage(page, { type: 'reload', fallbackUrl: page.pageView.webContents.getURL() })
    },
  )

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
