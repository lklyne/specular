/**
 * Test-only HTTP routes for smoke tests. Exposes minimal hooks into the
 * InteractionController, FocusReconciler, and DropOwner so smoke tests
 * can exercise the integrated state machine without simulating native
 * pointer events. Not for production use.
 *
 * Spec docs/interaction-layer.md §9 (testing strategy).
 */

import { writeJson } from './http-helpers'
import { app } from 'electron'
import {
  peek as peekInteractionMode,
  tryEnter,
  commit as commitInteraction,
  cancel as cancelInteraction,
  cancelActive,
  commitActive,
  __resetForTests as resetInteractionForTests,
  type TryEnterInput,
} from '../runtime/interaction-controller'
import { currentEditingEntityId } from '../runtime/editing-entity-runtime'
import {
  consumeDragId,
  __resetForTests as resetDropOwnerForTests,
} from '../runtime/drop-owner'
import { setPendingFocus, pages } from '../runtime/runtime-context'
import { requestLayout } from '../runtime/viewport-control'
import { aboveView, bgView, toolbarView, leftSidebarView } from '../runtime/view-refs'
import type { Route } from './types'
import type { Token, CancelReason, FocusTarget } from '../../shared/interaction-types'
import { activeTool } from '../runtime/tool-mode'
import { clipboard } from 'electron'
import { pasteFromClipboard } from '../clipboard-paste'
import {
  undo as undoCanvas,
  redo as redoCanvas,
  canUndo as canUndoCanvas,
  canRedo as canRedoCanvas,
  markUndoBoundary,
} from '../runtime/workspace-undo'
import { beginBatch, endBatch } from '../runtime/workspace-observers'
import { flushWorkspaceAutosaveSync, loadWorkspace } from '../runtime/workspace-autosave'
import {
  DEFAULT_WORKSPACE_ID,
  canvasFilePath,
  readCanvasFile,
  readWorkspaceMeta,
  writeCanvasFileSync,
  writeWorkspaceMetaSync,
} from '../runtime/workspace-persistence'
import { restorePersistedWorkspace } from '../runtime/workspace-restore'
import { getActiveDoc } from '../runtime/workspace-doc'
import { workspaceTabs, activeWorkspaceTabId } from '../runtime/workspace-model'
import { applyDragDelta, finalizeDrag, initializeDrag, resizeMultiSelection } from '../runtime/document-commands'
import type { MultiResizeEntry } from '../runtime/document-commands'
import { currentCanvasGuides } from '../runtime/canvas-guides'
import { currentEntityOrder, reorderSidebarStackOrder } from '../runtime/entity-order-state'
import type { SidebarSectionKey } from '../../shared/types'

// --- Y.Doc transaction counter (test-only) ---
// Counts afterTransaction events for the active doc between start/stop calls.
// Smoke tests use this to assert that a single mutation produces exactly one
// transaction (forward-sync echo regressions inflate the count).

let _transactionCount = 0
let _transactionCounterHandler: ((tx: { origin: unknown }) => void) | null = null

function startTransactionCounter(): void {
  if (_transactionCounterHandler) {
    getActiveDoc().off('afterTransaction', _transactionCounterHandler as never)
    _transactionCounterHandler = null
  }
  _transactionCount = 0
  const doc = getActiveDoc()
  const handler = () => {
    _transactionCount += 1
  }
  _transactionCounterHandler = handler
  doc.on('afterTransaction', handler)
}

function stopTransactionCounter(): number {
  if (_transactionCounterHandler) {
    getActiveDoc().off('afterTransaction', _transactionCounterHandler as never)
    _transactionCounterHandler = null
  }
  return _transactionCount
}

function currentlyFocusedKey(): string | null {
  if (bgView?.webContents.isFocused()) return 'bgView'
  if (aboveView?.webContents.isFocused()) return 'aboveView'
  if (toolbarView?.webContents.isFocused()) return 'toolbar'
  if (leftSidebarView?.webContents.isFocused()) return 'sidebar'
  for (const p of pages) {
    if (p.pageView.webContents.isFocused()) return `page:${p.id}`
  }
  return null
}

export const testRoutes: Route[] = [
  // --- InteractionController ---
  {
    method: 'GET',
    pattern: '/test/interaction/mode',
    async handler({ response }) {
      writeJson(response, 200, {
        mode: peekInteractionMode(),
        editingEntityId: currentEditingEntityId(),
      })
    },
  },
  {
    method: 'POST',
    pattern: '/test/interaction/begin',
    async handler({ response, body }) {
      const input = body as TryEnterInput
      const result = tryEnter(input)
      if ('refused' in result) {
        writeJson(response, 409, { refused: true, reason: result.reason })
        return
      }
      writeJson(response, 200, { token: result })
    },
  },
  {
    method: 'POST',
    pattern: '/test/interaction/commit',
    async handler({ response, body }) {
      const { token } = body as { token: Token }
      commitInteraction(token)
      writeJson(response, 200, { ok: true, mode: peekInteractionMode() })
    },
  },
  {
    method: 'POST',
    pattern: '/test/interaction/cancel',
    async handler({ response, body }) {
      const { token, reason } = body as { token: Token; reason: CancelReason }
      cancelInteraction(token, reason)
      writeJson(response, 200, { ok: true, mode: peekInteractionMode() })
    },
  },
  {
    method: 'POST',
    pattern: '/test/interaction/cancel-active',
    async handler({ response, body }) {
      const { reason } = body as { reason: CancelReason }
      cancelActive(reason)
      writeJson(response, 200, { ok: true, mode: peekInteractionMode() })
    },
  },
  {
    method: 'POST',
    pattern: '/test/interaction/reset',
    async handler({ response }) {
      resetInteractionForTests()
      writeJson(response, 200, { ok: true })
    },
  },

  // --- FocusReconciler ---
  {
    method: 'GET',
    pattern: '/test/focus/current',
    async handler({ response }) {
      writeJson(response, 200, { focused: currentlyFocusedKey() })
    },
  },
  {
    method: 'POST',
    pattern: '/test/focus/request',
    async handler({ response, body }) {
      const { target } = body as { target: FocusTarget }
      setPendingFocus(target)
      requestLayout()
      writeJson(response, 200, { ok: true })
    },
  },

  // --- DropOwner ---
  {
    method: 'POST',
    pattern: '/test/drop/consume-drag-id',
    async handler({ response, body }) {
      const { dragId } = body as { dragId: string }
      const wasConsumed = consumeDragId(dragId)
      writeJson(response, 200, { wasConsumed })
    },
  },
  {
    method: 'POST',
    pattern: '/test/drop/reset',
    async handler({ response }) {
      resetDropOwnerForTests()
      writeJson(response, 200, { ok: true })
    },
  },

  // --- Canvas drag simulation ---
  {
    method: 'POST',
    pattern: '/test/canvas-drag/start',
    async handler({ response, body }) {
      const { entityIds } = body as { entityIds: string[] }
      initializeDrag(entityIds)
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/test/canvas-drag/apply',
    async handler({ response, body }) {
      const {
        entityIds,
        dx,
        dy,
        shiftKey = false,
      } = body as { entityIds: string[]; dx: number; dy: number; shiftKey?: boolean }
      applyDragDelta(entityIds, dx, dy, { shiftKey })
      writeJson(response, 200, { ok: true, guides: currentCanvasGuides() })
    },
  },
  {
    method: 'POST',
    pattern: '/test/canvas-drag/end',
    async handler({ response }) {
      finalizeDrag()
      writeJson(response, 200, { ok: true, guides: currentCanvasGuides() })
    },
  },

  // --- Multi-selection resize simulation ---
  {
    method: 'POST',
    pattern: '/test/canvas-multi-resize/begin',
    async handler({ response }) {
      tryEnter({ kind: 'resizing-multi-selection' })
      beginBatch()
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/test/canvas-multi-resize/apply',
    async handler({ response, body }) {
      const { entries } = body as { entries: MultiResizeEntry[] }
      resizeMultiSelection(entries)
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/test/canvas-multi-resize/end',
    async handler({ response }) {
      endBatch()
      markUndoBoundary()
      commitActive()
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'GET',
    pattern: '/test/canvas-guides/current',
    async handler({ response }) {
      writeJson(response, 200, currentCanvasGuides())
    },
  },

  // --- Sidebar stack-order simulation ---
  {
    method: 'GET',
    pattern: '/test/workspace/entity-order',
    async handler({ response }) {
      writeJson(response, 200, { entityOrder: currentEntityOrder() })
    },
  },
  {
    method: 'POST',
    pattern: '/test/sidebar/reorder',
    async handler({ response, body }) {
      const payload = body as {
        section?: SidebarSectionKey
        draggedId?: string
        anchorId?: string | null
        position?: 'before' | 'after'
        parentId?: string | null
      }
      if (payload.section !== 'notes' && payload.section !== 'pages') {
        writeJson(response, 400, { error: 'section must be notes or pages' })
        return
      }
      if (!payload.draggedId) {
        writeJson(response, 400, { error: 'draggedId is required' })
        return
      }
      const ok = reorderSidebarStackOrder({
        section: payload.section,
        draggedId: payload.draggedId,
        anchorId: payload.anchorId ?? null,
        position: payload.position ?? 'before',
        parentId: payload.parentId ?? null,
      })
      writeJson(response, 200, { ok, entityOrder: currentEntityOrder() })
    },
  },
  {
    method: 'POST',
    pattern: '/test/workspace/load-canvas-fixture',
    async handler({ response, body }) {
      const payload = body as {
        name?: string
        doc?: Parameters<typeof writeCanvasFileSync>[1]
      }
      if (!payload.doc || !Array.isArray(payload.doc.nodes) || !Array.isArray(payload.doc.edges)) {
        writeJson(response, 400, { error: 'doc with nodes and edges is required' })
        return
      }

      const userDataPath = app.getPath('userData')
      const name = payload.name?.trim() || 'Fixture'
      const tabId = 'fixture-tab'
      writeCanvasFileSync(canvasFilePath(userDataPath, DEFAULT_WORKSPACE_ID, name), payload.doc)
      writeWorkspaceMetaSync(userDataPath, DEFAULT_WORKSPACE_ID, {
        activeTabId: tabId,
        viewMode: 'canvas',
        tabs: [{
          id: tabId,
          name,
          updatedAt: new Date().toISOString(),
          expanded: true,
        }],
      })

      const record = loadWorkspace()
      if (!record || !restorePersistedWorkspace(record)) {
        writeJson(response, 500, { error: 'failed to load fixture workspace' })
        return
      }

      requestLayout()
      writeJson(response, 200, { ok: true, entityOrder: currentEntityOrder() })
    },
  },

  // --- Tool state ---
  {
    method: 'GET',
    pattern: '/test/tool/current',
    async handler({ response }) {
      writeJson(response, 200, { tool: activeTool() })
    },
  },

  // --- Clipboard simulation ---
  // Tests write text to the system clipboard and trigger smart-paste so they
  // can exercise URL → page, plain text → sticky, etc., without IPC plumbing.
  {
    method: 'POST',
    pattern: '/test/clipboard/paste',
    async handler({ response, body }) {
      const { text, canvasX = 0, canvasY = 0 } = body as {
        text?: string
        canvasX?: number
        canvasY?: number
      }
      if (typeof text === 'string') clipboard.writeText(text)
      pasteFromClipboard({ canvasX, canvasY })
      writeJson(response, 200, { ok: true })
    },
  },

  // --- Workspace undo/redo (test-only triggers; production goes through key bindings) ---
  {
    method: 'POST',
    pattern: '/test/workspace/undo',
    async handler({ response }) {
      undoCanvas()
      writeJson(response, 200, { ok: true, canUndo: canUndoCanvas(), canRedo: canRedoCanvas() })
    },
  },
  {
    method: 'POST',
    pattern: '/test/workspace/redo',
    async handler({ response }) {
      redoCanvas()
      writeJson(response, 200, { ok: true, canUndo: canUndoCanvas(), canRedo: canRedoCanvas() })
    },
  },
  {
    method: 'GET',
    pattern: '/test/workspace/undo-state',
    async handler({ response }) {
      writeJson(response, 200, { canUndo: canUndoCanvas(), canRedo: canRedoCanvas() })
    },
  },

  // --- Autosave + persistence inspection ---
  {
    method: 'POST',
    pattern: '/test/workspace/flush-autosave',
    async handler({ response }) {
      flushWorkspaceAutosaveSync()
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'GET',
    pattern: '/test/workspace/disk-snapshot',
    async handler({ request, response }) {
      const url = new URL(request.url ?? '/', 'http://x')
      const requestedTabId = url.searchParams.get('tabId')
      const userDataPath = app.getPath('userData')
      const meta = readWorkspaceMeta(userDataPath, DEFAULT_WORKSPACE_ID)
      if (!meta) {
        writeJson(response, 200, { exists: false, meta: null, tab: null })
        return
      }
      const tabId = requestedTabId ?? meta.activeTabId
      const tabMeta = meta.tabs.find((t) => t.id === tabId) ?? meta.tabs[0]
      if (!tabMeta) {
        writeJson(response, 200, { exists: true, meta, tab: null })
        return
      }
      const filePath = canvasFilePath(userDataPath, DEFAULT_WORKSPACE_ID, tabMeta.name)
      const doc = readCanvasFile(filePath)
      writeJson(response, 200, {
        exists: doc !== null,
        meta,
        tab: tabMeta,
        filePath,
        doc,
      })
    },
  },

  // --- Y.Doc transaction counter (one mutation should produce exactly one transaction) ---
  {
    method: 'POST',
    pattern: '/test/workspace/transactions/start',
    async handler({ response }) {
      startTransactionCounter()
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/test/workspace/transactions/stop',
    async handler({ response }) {
      writeJson(response, 200, { count: stopTransactionCounter() })
    },
  },

  // --- Workspace tab state introspection ---
  {
    method: 'GET',
    pattern: '/test/workspace/tabs',
    async handler({ response }) {
      writeJson(response, 200, {
        activeTabId: activeWorkspaceTabId,
        tabs: workspaceTabs.map((t) => ({ id: t.id, name: t.name })),
      })
    },
  },

  // --- Keyboard simulation ---
  {
    method: 'POST',
    pattern: '/test/keyboard/send',
    async handler({ response, body }) {
      const {
        key,
        cmd = false,
        shift = false,
        alt = false,
        target = 'aboveView',
        pageId,
      } = body as {
        key: string
        cmd?: boolean
        shift?: boolean
        alt?: boolean
        target?: 'aboveView' | 'bgView' | 'toolbar' | 'page'
        pageId?: string
      }
      let wc: Electron.WebContents | undefined
      if (target === 'page') {
        const page = pages.find((p) => p.id === pageId)
        wc = page?.pageView.webContents
      } else if (target === 'bgView') {
        wc = bgView?.webContents
      } else if (target === 'toolbar') {
        wc = toolbarView?.webContents
      } else {
        wc = aboveView?.webContents
      }
      if (!wc || wc.isDestroyed()) {
        writeJson(response, 500, { error: 'target webContents unavailable' })
        return
      }
      // sendInputEvent triggers before-input-event in the main process, which
      // the binding dispatcher picks up — no actual key is delivered to the page.
      // Use 'meta' for cmd since Electron runs on macOS in this project.
      const modifiers: string[] = []
      if (cmd) modifiers.push('meta')
      if (shift) modifiers.push('shift')
      if (alt) modifiers.push('alt')
      wc.sendInputEvent({ type: 'keyDown', keyCode: key, modifiers } as any)
      wc.sendInputEvent({ type: 'keyUp', keyCode: key, modifiers } as any)
      writeJson(response, 200, { ok: true })
    },
  },
]
