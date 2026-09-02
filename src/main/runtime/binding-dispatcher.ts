// fallow-ignore-file circular-dependencies
// Suppressed: see #141. binding-handlers → runtime-core → page-factory/window-init import binding-dispatcher back
import { ipcChannels } from '../../shared/ipc-contract'
import type { Tool } from '../../shared/tool'
import type { WebContents } from 'electron'
import {
  BINDINGS,
  CANVAS_REGION,
  dispatchKey,
  normalizeElectronInput,
  type BindingContext,
  type KeyboardSourceView,
} from '../../shared/bindings'
import { activeTool, setActiveTool } from './tool-mode'
import { aboveView } from './view-refs'
import { mainHandlers } from './binding-handlers'
import { hasFocusReturnCamera } from './viewport-control'

// Track text-editing state per webContents. A keystroke can only land in the
// webContents that has focus, so dispatch consults that source's flag — not
// a global aggregate. Aggregating let unrelated webContents (e.g. a page
// with an autofocused input) suppress canvas-region shortcuts.
const textEditingByWebContents = new WeakMap<WebContents, boolean>()

// Annotation state surfaced from above-view's renderer-local React state so
// Escape resolution (annotation-close-thread / annotation-clear-draft) works.
let hasOpenAnnotationThread = false
let hasPendingAnnotation = false

export function setAnnotationState(openThread: boolean, pendingAnnotation: boolean): void {
  hasOpenAnnotationThread = openThread
  hasPendingAnnotation = pendingAnnotation
}

export function setTextEditingActive(webContents: WebContents, active: boolean): void {
  const prev = textEditingByWebContents.get(webContents) ?? false
  if (prev === active) return
  textEditingByWebContents.set(webContents, active)
}

export function isTextEditingFor(webContents: WebContents): boolean {
  return textEditingByWebContents.get(webContents) ?? false
}

export function buildBindingContext(
  sourceView: KeyboardSourceView,
  pageFocusActive: boolean,
  isTextEditing = false,
): BindingContext {
  return {
    activeTool: activeTool(),
    isTextEditing,
    pageFocusActive,
    focusReturnCameraActive: hasFocusReturnCamera(),
    sourceView,
    hasOpenAnnotationThread,
    hasPendingAnnotation,
  }
}

const attachedWebContents = new WeakSet<WebContents>()

// Hold Space to momentarily switch to the hand tool; release to restore the
// prior tool. Keyup can't be a binding (the table is keydown-only), so this
// lives on the raw before-input-event stream. Non-null while held.
let toolBeforeSpacePan: Tool | null = null

function handleSpacePanToggle(
  isDown: boolean,
  sourceView: KeyboardSourceView,
  webContents: WebContents,
): void {
  if (isDown) {
    // Space types a space in a text field and scrolls a focused page; only
    // hijack it over the canvas surface.
    if (!CANVAS_REGION.includes(sourceView)) return
    if (isTextEditingFor(webContents)) return
    // Guarding on the live tool (not the latch flag) covers both autorepeat
    // keyDowns and self-heals a stale latch left by a keyup lost to a
    // window-focus change: the next real press re-latches from the true tool.
    const prev = activeTool()
    if (prev.kind === 'hand') return
    toolBeforeSpacePan = prev
    setActiveTool({ kind: 'hand' })
    return
  }
  // Keyup restores unconditionally — if we never latched, this is a no-op.
  if (!toolBeforeSpacePan) return
  const restore = toolBeforeSpacePan
  toolBeforeSpacePan = null
  setActiveTool(restore)
}

export function attachBindingDispatcher(
  webContents: WebContents,
  sourceView: KeyboardSourceView,
): void {
  if (attachedWebContents.has(webContents)) return
  attachedWebContents.add(webContents)

  webContents.on('destroyed', () => {
    textEditingByWebContents.delete(webContents)
  })

  webContents.on('before-input-event', (event, input) => {
    // Track Space modifier regardless of editing state — space-to-pan must
    // stay in sync even when focus is in an input that consumes Space natively.
    if (input.key === ' ' || input.code === 'Space') {
      handleSpacePanToggle(input.type === 'keyDown', sourceView, webContents)
    }

    const normalizedKey = normalizeElectronInput(input)
    if (!normalizedKey) return

    const pageFocusActive = sourceView === 'page'
    const ctx = buildBindingContext(
      sourceView,
      pageFocusActive,
      isTextEditingFor(webContents),
    )

    const bindingId = dispatchKey(BINDINGS, normalizedKey, ctx)
    if (!bindingId) return

    const binding = BINDINGS.find((b) => b.id === bindingId && b.defaultKey.key === normalizedKey.key)
    if (!binding) return

    event.preventDefault()

    if (binding.target === 'main') {
      const handler = (mainHandlers as Record<string, ((ctx: BindingContext) => void) | undefined>)[bindingId]
      if (handler) handler(ctx)
    } else {
      // Renderer-targeted binding — forward via IPC
      const targetWc = resolveTargetWebContents(binding.target)
      if (targetWc && !targetWc.isDestroyed()) {
        targetWc.send(ipcChannels.bindingFire, bindingId)
      }
    }
  })
}

function resolveTargetWebContents(target: string): WebContents | null {
  if (target === 'aboveView') return aboveView?.webContents ?? null
  return null
}
