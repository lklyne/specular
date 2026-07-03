import { ipcChannels } from '../../shared/ipc-contract'
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
  toggleSvgDeviceShell,
  setFilePreset,
  setFileCustom,
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

/** Channels that validate one id field, then call one function. */
const SINGLE_FIELD_COMMANDS: Record<string, SingleFieldCommand> = {
  // --- Annotations and fixes ---
  [ipcChannels.rightDetailsPanelResolveAnnotation]: {
    key: 'annotationId',
    trim: true,
    run: (id) => updateAnnotationStatus(id, 'resolved'),
  },
  [ipcChannels.rightDetailsPanelDeleteAnnotation]: {
    key: 'annotationId',
    trim: true,
    run: (id) => deleteAnnotation(id),
  },
  [ipcChannels.rightDetailsPanelTriggerFixComments]: {
    key: 'origin',
    trim: true,
    run: (origin) => fixPendingAnnotationsForOrigin(origin),
  },
  [ipcChannels.rightDetailsPanelFixSingleAnnotation]: {
    key: 'annotationId',
    trim: true,
    run: (id) => fixAnnotation(id),
  },
  // --- Device Page ---
  [ipcChannels.rightDetailsPanelSetPagePreset]: {
    key: 'pageId',
    accept: hasPresetIndex,
    run: (id, payload) => setPagePreset(id, payload.presetIndex as number),
  },
  [ipcChannels.rightDetailsPanelToggleSvgDeviceShell]: {
    key: 'pageId',
    run: (id) => toggleSvgDeviceShell(id),
  },
  // --- File Device Settings ---
  [ipcChannels.rightDetailsPanelSetFilePreset]: {
    key: 'fileId',
    accept: hasPresetIndex,
    run: (id, payload) => setFilePreset(id, payload.presetIndex as number),
  },
  [ipcChannels.rightDetailsPanelSetFileCustom]: {
    key: 'fileId',
    run: (id) => setFileCustom(id),
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
  ipcMain.on(ipcChannels.rightDetailsPanelOpenBrowserDevtools, () => {
    openDevToolsForSelectedPage()
  })

  ipcMain.on(ipcChannels.rightDetailsPanelDismissBrowserDevtools, () => {
    dismissBrowserDevTools()
  })

  ipcMain.on(ipcChannels.rightDetailsPanelClearInspectSelection, () => {
    setSelectedInspectTarget(null)
    markDirty('canvas')
    requestLayout()
  })

  ipcMain.on(
    ipcChannels.rightDetailsPanelSelectPage,
    (_event, payload: { pageId?: string } | undefined) => {
      const pageId = payload?.pageId?.trim()
      if (!pageId) return
      selectPageById(pageId)
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelHoverNode,
    (_event, { pageId, nodeId }: { pageId: string; nodeId: string | null }) => {
      if (!pageId) return
      setInspectNodeFromPanel(pageId, nodeId, false)
      markDirty('canvas')
      requestLayout()
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelSelectNode,
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
    ipcChannels.rightDetailsPanelEditComponentProp,
    (
      _event,
      { pageId, componentId, propPath, value }: ComponentPropOverridePayload,
    ) => {
      forwardOverrideToPage(pageId, ipcChannels.overrideProps, {
        componentId,
        propPath,
        value,
      })
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelEditComponentToken,
    (
      _event,
      { pageId, componentId, token, value, selector }: ComponentTokenOverridePayload,
    ) => {
      forwardOverrideToPage(pageId, ipcChannels.overrideToken, {
        componentId,
        token,
        value,
        selector,
      })
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelCreateAnnotation,
    (_event, request: AnnotationCreateRequest) => {
      createAnnotation(request)
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelReplyAnnotation,
    (_event, payload: { annotationId?: string; text?: string } | undefined) => {
      const annotationId = payload?.annotationId?.trim()
      const text = payload?.text?.trim()
      if (!annotationId || !text) return
      addAnnotationReply(annotationId, 'user', text)
    },
  )

  registerSingleFieldHandlers()

  ipcMain.on(
    ipcChannels.rightDetailsPanelSetAutoFix,
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
    ipcChannels.rightDetailsPanelPickRepoForOrigin,
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
    ipcChannels.rightDetailsPanelRemoveOriginBinding,
    (_event, payload: { origin?: string } | undefined) => {
      const origin = payload?.origin?.trim()
      if (!origin) return
      if (removeBindingByOrigin(origin)) notifyDevtoolsPanelData()
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelSetFixConfig,
    (_event, payload: { model?: string; permissions?: string } | undefined) => {
      if (!payload) return
      setFixConfig(payload as { model?: 'opus' | 'sonnet' | 'haiku'; permissions?: 'dangerously' | 'default' })
      notifyDevtoolsPanelData()
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelUpdateEdge,
    (_event, payload: { id: string; patch: { fromEnd?: EdgeEnd; toEnd?: EdgeEnd; fromSide?: EdgeSide; toSide?: EdgeSide; color?: string; label?: string } }) => {
      if (!payload?.id) return
      updateEdge(payload.id, payload.patch)
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelDeleteEdge,
    (_event, payload: { id: string }) => {
      if (!payload?.id) return
      deleteEdge(payload.id)
    },
  )

  // --- Page actions ---

  ipcMain.on(
    ipcChannels.rightDetailsPanelDuplicatePage,
    (_event, payload: { pageId: string }) => {
      if (!payload?.pageId) return
      if (!pages.some((p) => p.id === payload.pageId)) return
      duplicatePageFromSource({ sourcePageId: payload.pageId })
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelToggleLinkedPage,
    (_event, payload: { pageId: string }) => {
      const page = pages.find((p) => p.id === payload?.pageId)
      if (!page) return
      togglePageLinked(page)
    },
  )

  ipcMain.on(
    ipcChannels.rightDetailsPanelDeletePage,
    (_event, payload: { pageId: string }) => {
      if (!payload?.pageId) return
      if (!pages.some((p) => p.id === payload.pageId)) return
      deletePages({ pageIds: [payload.pageId] })
    },
  )
}
