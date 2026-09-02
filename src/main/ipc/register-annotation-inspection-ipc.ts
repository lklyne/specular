import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type { Annotation, ComponentTreeNode, WorkspaceBounds } from '../../shared/types'
import { aboveView } from '../runtime/view-refs'
import {
  pageBodyCanvasBounds,
  projectFramePointToCanvas,
} from '../runtime/runtime-geometry'
import {
  findPageById,
  findPageByPageView,
  getComponentSourceLocationByNodeId,
  handlePageIpcResponse,
  handleNodeDetailResponse,
} from '../runtime/page-runtime'
import { regionCanvasRect } from '../runtime/page-anchor-state'
import { dispatchScrollToAnnotation } from '../runtime/annotation-scroll-target'
import { revealPageAnchoredContent } from '../runtime/page-anchor-reveal'
import { broadcastInspectSlice } from '../runtime/inspect-session'
import { requestLayout } from '../runtime/viewport-control'
import {
  focusCanvasBounds,
  openCommentsPanel,
  openInspectPanel,
  focusAnnotation,
  selectPageById,
  setHoveredInspectTarget,
  setSelectedInspectTarget,
} from '../runtime/ui-actions'
import { setCommentOverlayActive } from '../runtime/window-shell'
import { getAnnotationById } from '../workspace-annotations'
import {
  forwardOverrideToPage,
  type ComponentPropOverridePayload,
  type ComponentTokenOverridePayload,
} from './component-override'

const POINT_FOCUS_SIZE = 100

function annotationCanvasBounds(annotation: Annotation): WorkspaceBounds | null {
  const { anchor } = annotation
  switch (anchor.type) {
    case 'canvas':
      return {
        x: anchor.canvasX - POINT_FOCUS_SIZE / 2,
        y: anchor.canvasY - POINT_FOCUS_SIZE / 2,
        width: POINT_FOCUS_SIZE,
        height: POINT_FOCUS_SIZE,
      }
    case 'region':
      // A page-anchored region stores a document rect; resolve it to where the
      // region sits on the canvas right now (tracks page move + scroll).
      return regionCanvasRect(annotation)
    case 'element': {
      const page = findPageById(anchor.pageId)
      if (!page) return null
      if (anchor.boundingBox) {
        const origin = projectFramePointToCanvas(page, anchor.boundingBox)
        return {
          x: origin.x,
          y: origin.y,
          width: anchor.boundingBox.width,
          height: anchor.boundingBox.height,
        }
      }
      return pageBodyCanvasBounds(page)
    }
    case 'page': {
      const page = findPageById(anchor.pageId)
      if (!page) return null
      return pageBodyCanvasBounds(page)
    }
  }
}

export function registerAnnotationInspectionIpc(): void {
  ipcMain.on(
    ipcChannels.annotationOpenThread,
    (_event, payload: { annotationId?: string | null; reveal?: boolean } | undefined) => {
      const annotationId =
        typeof payload?.annotationId === 'string' && payload.annotationId.trim().length > 0
          ? payload.annotationId
          : null
      if (!annotationId) {
        // Back out of the focused thread: the panel returns to the comments
        // list and the canvas ring clears.
        focusAnnotation(undefined)
        if (aboveView && !aboveView.webContents.isDestroyed()) {
          aboveView.webContents.send(ipcChannels.annotationThreadOpen, {
            annotationId: null,
          })
        }
        return
      }
      const annotation = getAnnotationById(annotationId)
      if (!annotation) return
      const pageAnchor = annotation.pageAnchor
      if (pageAnchor) selectPageById(pageAnchor.pageId)
      // A click on the annotation itself (canvas badge, region overlay) passes
      // reveal: false — the anchor is already in view, so a camera move would
      // only jolt the user. Panel and sidebar rows keep the reveal.
      const reveal = payload?.reveal !== false
      if (reveal) {
        const bounds = annotationCanvasBounds(annotation)
        if (bounds) focusCanvasBounds(bounds)
      }
      // The conversation lives in the right panel: open it (if closed), switch
      // to comments, and focus this thread. The canvas keeps only the ring,
      // painted by aboveView off the echo below.
      openCommentsPanel(annotationId)
      requestLayout()
      if (aboveView && !aboveView.webContents.isDestroyed()) {
        aboveView.webContents.send(ipcChannels.annotationThreadOpen, {
          annotationId,
        })
      }
      // Reveal the commented content on the page itself: canvas focus alone
      // leaves a long page pointing at content the user can't see. Fire-and-
      // forget; no-op for canvas points and canvas-anchored regions, which mark
      // canvas space, not page content (ADR 0029).
      if (reveal) {
        revealPageAnchoredContent(pageAnchor, () =>
          dispatchScrollToAnnotation(annotation),
        )
      }
    },
  )

  ipcMain.on(ipcChannels.inspectNodeHover, (event, payload) => {
    const page = findPageByPageView(event.sender)
    if (!page) return
    if (!payload || typeof payload !== 'object') {
      setHoveredInspectTarget(null)
      broadcastInspectSlice()
      requestLayout()
      return
    }
    setHoveredInspectTarget({
      ...payload,
      pageId: page.id,
    })
    broadcastInspectSlice()
    requestLayout()
  })

  ipcMain.on(ipcChannels.inspectNodeSelect, (event, payload) => {
    const page = findPageByPageView(event.sender)
    if (!page) return
    if (!payload || typeof payload !== 'object') {
      setSelectedInspectTarget(null)
      return
    }
    selectPageById(page.id)
    openInspectPanel()
    setSelectedInspectTarget({
      ...payload,
      pageId: page.id,
    })
    broadcastInspectSlice()
    requestLayout()
  })

  ipcMain.on(ipcChannels.inspectNodeDetailUpdate, (event, payload) => {
    const page = findPageByPageView(event.sender)
    if (!page || !payload || typeof payload !== 'object') return
    const raw = payload as { nodeId?: string; id?: string }
    const nodeId = raw.nodeId ?? raw.id
    if (!nodeId) return
    page.inspectDetailsByNodeId ??= {}
    page.inspectDetailsByNodeId[nodeId] = {
      ...(payload as Record<string, unknown>),
      nodeId,
      id: nodeId,
      pageId: page.id,
      sourceLocation: getComponentSourceLocationByNodeId(page.id, nodeId),
    } as NonNullable<typeof page.inspectDetailsByNodeId>[string]
  })

  ipcMain.on(ipcChannels.resolveNodeDetailResponse, (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handleNodeDetailResponse(payload)
  })

  ipcMain.on(ipcChannels.takeDomSnapshotResponse, (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on(ipcChannels.queryDomElementsResponse, (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on(ipcChannels.queryElementsInRectResponse, (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on(ipcChannels.queryElementAtPointResponse, (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on(ipcChannels.captureElementAtPointResponse, (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on(ipcChannels.dispatchScrollResult, (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on(ipcChannels.queryActiveElementRectResult, (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on(ipcChannels.inspectTreeUpdate, (event, payload) => {
    const page = findPageByPageView(event.sender)
    if (!page || !Array.isArray(payload)) return
    page.componentTree = payload as ComponentTreeNode[]
  })

  ipcMain.on(
    ipcChannels.canvasEditComponentProp,
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
    ipcChannels.canvasEditComponentToken,
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

  ipcMain.on(ipcChannels.commentOverlaySetActive, (_event, active: boolean) => {
    setCommentOverlayActive(Boolean(active))
  })

  // ADR 0006 retired the page-side `annotate-element-select` self-firing
  // path. Element resolution for the comment tool now happens via
  // `query-element-at-point` invoked from `canvas-comment-click-at` —
  // see `register-canvas-entity-ipc.ts`. The `annotate-element-selected`
  // channel sent to aboveView is unchanged.
}
