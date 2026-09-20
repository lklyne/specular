import { useCallback, useEffect, useRef } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { isOverlayUiTarget, isTypingTarget } from '../../shared/gesture-utils'
import { textForKeyPress, type ForwardKeyPayload } from '../../shared/page-key-input'

export interface UsePageKeyboardForwardingOptions {
  api: CanvasBgElectronAPI
  /** The page keys belong to (`focus` slice), or null for the canvas. */
  keyboardTargetPageId: string | null
  /** Non-null while an inline canvas editor holds the keyboard. */
  editingEntityId: string | null
}

export interface PageKeyboardForwarding {
  sinkRef: React.RefObject<HTMLInputElement | null>
  /** Move keyboard focus into the sink, unless a real editor already has it. */
  focusSink: () => void
}

/**
 * aboveView's keyboard sink — the hidden input that stands in for a page's own
 * keyboard focus.
 *
 * A page renders offscreen, so OS key events go to the focused native view and
 * never to the page. aboveView owns that focus always; a keystroke lands in
 * this input, main's binding dispatcher gets first refusal over
 * `before-input-event`, and whatever it does not claim reaches the DOM here and
 * is forwarded into the page over CDP.
 *
 * Live composition UI is an accepted loss (Chromium's offscreen widget host
 * makes `TextInputStateChanged` a no-op): the commit arrives whole on
 * `compositionend` and goes in as `Input.insertText`.
 */
export function usePageKeyboardForwarding({
  api,
  keyboardTargetPageId,
  editingEntityId,
}: UsePageKeyboardForwardingOptions): PageKeyboardForwarding {
  const sinkRef = useRef<HTMLInputElement | null>(null)
  const pageIdRef = useRef(keyboardTargetPageId)
  pageIdRef.current = keyboardTargetPageId
  const editingRef = useRef(editingEntityId)
  editingRef.current = editingEntityId

  const focusSink = useCallback(() => {
    const sink = sinkRef.current
    if (!sink) return
    const active = document.activeElement
    if (active === sink) return
    // An inline editor or an overlay-UI field owns the keyboard while it is
    // open; taking it would swallow what is being typed there.
    if (active && (isTypingTarget(active) || isOverlayUiTarget(active))) return
    sink.focus({ preventScroll: true })
  }, [])

  useEffect(() => {
    const sink = sinkRef.current
    if (!sink) return
    if (keyboardTargetPageId) {
      focusSink()
      return
    }
    if (document.activeElement === sink) sink.blur()
  }, [focusSink, keyboardTargetPageId])

  useEffect(() => {
    const sink = sinkRef.current
    if (!sink) return

    const forwardKey = (event: KeyboardEvent, kind: ForwardKeyPayload['kind']) => {
      const pageId = pageIdRef.current
      if (!pageId || editingRef.current) return
      // keyCode 229 is the placeholder a composing IME reports; the commit
      // arrives on compositionend instead.
      if (event.isComposing || event.keyCode === 229) return
      event.preventDefault()
      api.forwardKeyToPage(pageId, {
        kind,
        key: event.key,
        code: event.code,
        text: kind === 'down' ? textForKeyPress(event) : null,
        repeat: event.repeat,
        shiftKey: event.shiftKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        metaKey: event.metaKey,
      })
    }

    const onKeyDown = (event: KeyboardEvent) => forwardKey(event, 'down')
    const onKeyUp = (event: KeyboardEvent) => forwardKey(event, 'up')
    const onCompositionEnd = (event: CompositionEvent) => {
      const pageId = pageIdRef.current
      if (pageId && !editingRef.current && event.data) {
        api.insertTextIntoPage(pageId, event.data)
      }
      sink.value = ''
    }

    sink.addEventListener('keydown', onKeyDown)
    sink.addEventListener('keyup', onKeyUp)
    sink.addEventListener('compositionend', onCompositionEnd)
    return () => {
      sink.removeEventListener('keydown', onKeyDown)
      sink.removeEventListener('keyup', onKeyUp)
      sink.removeEventListener('compositionend', onCompositionEnd)
    }
  }, [api])

  return { sinkRef, focusSink }
}
