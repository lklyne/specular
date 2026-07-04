import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type { Annotation, ComponentTreeNode, WorkspaceBounds } from '../../shared/types'
import { bgView, aboveView } from '../runtime/view-refs'
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
import { getZoom, setPendingFocus } from '../runtime/runtime-context'
import { requestLayout, setZoom } from '../runtime/viewport-control'
import {
  focusCanvasBounds,
  getSelectedEntityIds,
  openCommentsPanel,
  openInspectPanel,
  focusAnnotation,
  selectPageById,
  setHoveredInspectTarget,
  setSelectedInspectTarget,
} from '../runtime/ui-actions'
import { setCommentOverlayActive } from '../runtime/window-shell'
import { getAnnotationById } from '../workspace-annotations'
import { markDirty } from '../runtime/layout-dirty'
import {
  forwardOverrideToPage,
  type ComponentPropOverridePayload,
  type ComponentTokenOverridePayload,
} from './component-override'

const POINT_FOCUS_SIZE = 100
const FOCUS_MIN_ZOOM = 0.8

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
      return anchor.canvasRect
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
    (_event, payload: { annotationId?: string } | undefined) => {
      const annotationId =
        typeof payload?.annotationId === 'string' && payload.annotationId.trim().length > 0
          ? payload.annotationId
          : null
      if (!annotationId) return
      const annotation = getAnnotationById(annotationId)
      if (!annotation) return
      if (annotation.anchor.type !== 'canvas' && annotation.anchor.type !== 'region') {
        selectPageById(annotation.anchor.pageId)
      }
      if (getZoom() < FOCUS_MIN_ZOOM) setZoom(1.0)
      const bounds = annotationCanvasBounds(annotation)
      if (bounds) focusCanvasBounds(bounds)
      focusAnnotation(annotationId)
      setCommentOverlayActive(true)
      setPendingFocus({ kind: 'aboveView' })
      requestLayout()
      if (aboveView && !aboveView.webContents.isDestroyed()) {
        aboveView.webContents.send(ipcChannels.annotationThreadOpen, {
          annotationId,
        })
      }
    },
  )

  ipcMain.on(ipcChannels.inspectNodeHover, (event, payload) => {
    const page = findPageByPageView(event.sender)
    if (!page) return
    if (!payload || typeof payload !== 'object') {
      setHoveredInspectTarget(null)
      markDirty('canvas')
      requestLayout()
      return
    }
    setHoveredInspectTarget({
      ...payload,
      pageId: page.id,
    })
    markDirty('canvas')
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
    markDirty('canvas')
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
    if (bgView && !bgView.webContents.isDestroyed()) {
      const selectedIds = getSelectedEntityIds()
      if (selectedIds.length === 1 && selectedIds[0] === page.id) {
        bgView.webContents.send(ipcChannels.componentTreeData, {
          pageId: page.id,
          tree: page.componentTree,
        })
      }
    }
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
