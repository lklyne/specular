import { ipcMain } from 'electron'
import type { Annotation, ComponentTreeNode, WorkspaceBounds } from '../../shared/types'
import {
  bgView,
  aboveView,
  pageBodyCanvasBounds,
  projectFramePointToCanvas,
  requestLayout,
} from '../runtime/surface-layout'
import {
  findPageById,
  findPageByPageView,
  getComponentSourceLocationByNodeId,
  handlePageIpcResponse,
  handleNodeDetailResponse,
  pages,
} from '../runtime/page-runtime'
import { getZoom, setPendingFocus } from '../runtime/runtime-context'
import { setZoom } from '../runtime/viewport-control'
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
    'annotation-open-thread',
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
        aboveView.webContents.send('annotation-thread-open', {
          annotationId,
        })
      }
    },
  )

  ipcMain.on('inspect-node-hover', (event, payload) => {
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

  ipcMain.on('inspect-node-select', (event, payload) => {
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

  ipcMain.on('inspect-node-detail-update', (event, payload) => {
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

  ipcMain.on('resolve-node-detail-response', (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handleNodeDetailResponse(payload)
  })

  ipcMain.on('take-dom-snapshot-response', (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on('query-dom-elements-response', (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on('query-elements-in-rect-response', (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on('query-element-at-point-response', (_event, payload) => {
    if (!payload || typeof payload !== 'object') return
    handlePageIpcResponse(payload as { requestId: string; data: unknown })
  })

  ipcMain.on('inspect-tree-update', (event, payload) => {
    const page = findPageByPageView(event.sender)
    if (!page || !Array.isArray(payload)) return
    page.componentTree = payload as ComponentTreeNode[]
    if (bgView && !bgView.webContents.isDestroyed()) {
      const selectedIds = getSelectedEntityIds()
      if (selectedIds.length === 1 && selectedIds[0] === page.id) {
        bgView.webContents.send('component-tree-data', {
          pageId: page.id,
          tree: page.componentTree,
        })
      }
    }
  })

  ipcMain.on(
    'canvas-edit-component-prop',
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
    'canvas-edit-component-token',
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

  ipcMain.on('comment-overlay-set-active', (_event, active: boolean) => {
    setCommentOverlayActive(Boolean(active))
  })

  // ADR 0006 retired the page-side `annotate-element-select` self-firing
  // path. Element resolution for the comment tool now happens via
  // `query-element-at-point` invoked from `canvas-comment-click-at` —
  // see `register-canvas-entity-ipc.ts`. The `annotate-element-selected`
  // channel sent to aboveView is unchanged.
}
