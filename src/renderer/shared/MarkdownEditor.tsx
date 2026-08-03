import { useEffect, useRef } from 'react'
import { Compartment, EditorState, type Extension } from '@codemirror/state'
import { EditorView } from '@codemirror/view'
import {
  createMarkdownExtensions,
  createStickyTextExtensions,
  externalUpdate,
  reconfigureTheme,
} from './markdown/markdown-codemirror'
import { placeholderExtension } from './markdown/markdown-placeholder'
import { autofocusEditorSelection } from '../../shared/editor-selection'

/**
 * CodeMirror-based markdown editor. Renders the source as live-styled
 * markdown (heading sizes, bold, links) so edit and view modes share the
 * same visual metrics — see MARKDOWN_TOKENS in markdown-codemirror.ts.
 *
 * `readOnly` renders the same view non-editable, so a text body can be
 * displayed by this component in both view and edit mode — one set of
 * padding and line boxes, so the mode swap can't reflow the text. Read-only
 * mode also leaves pointer events alone so the canvas router still gets the
 * drag.
 *
 * While editable the host stops mousedown propagation so the canvas pointer
 * router doesn't treat clicks inside the editor as canvas drags.
 *
 * `readOnly` swaps live through a CodeMirror compartment — the view is never
 * torn down for a mode change. A remount would blank the note for a frame
 * (the view is built in an effect, so the browser paints the empty container
 * first) and would throw away scroll position and the parsed document.
 */
export interface MarkdownEditorProps {
  value: string
  onChange: (value: string) => void
  onFocus?: () => void
  onBlur?: () => void
  /** Called when the user presses Escape inside the editor. */
  onEscape?: () => void
  /** Called with a link's URL: Cmd+click while editing, plain click read-only. */
  onOpenLink?: (url: string) => void
  isDark: boolean
  autoFocus?: boolean
  placeholder?: string
  className?: string
  style?: React.CSSProperties
  /** Disable to let the editor's container expand horizontally to fit content. */
  lineWrap?: boolean
  /** Select the full value when auto-focusing; intended for short text nodes. */
  selectAllOnAutoFocus?: boolean
  /** Display the value without a caret, focus, or text selection. Swaps live. */
  readOnly?: boolean
  /** Which CodeMirror extension stack to mount. Read at mount only — an
   *  entity never changes variant. Defaults to the full markdown stack. */
  variant?: 'markdown' | 'sticky'
  /** Called with the live EditorView right after creation, and with `null`
   *  in the unmount cleanup before it's destroyed. Lets a host drive
   *  formatting commands against the view directly. */
  onViewReady?: (view: EditorView | null) => void
  /** Called on every selection/doc change with the live EditorState — lets a
   *  host derive cursor-relative UI (e.g. active formatting state). */
  onSelectionChange?: (state: EditorState) => void
}

export function MarkdownEditor(props: MarkdownEditorProps) {
  const { className, style, onOpenLink, readOnly = false } = props
  const containerRef = useRef<HTMLDivElement | null>(null)
  useMarkdownEditor(containerRef, props)
  const readOnlyLink = useReadOnlyLinkClicks(readOnly ? onOpenLink : undefined)

  return (
    <div
      ref={containerRef}
      className={className}
      style={readOnly ? { ...style, userSelect: 'none' } : style}
      onPointerDown={readOnly ? readOnlyLink.onPointerDown : (e) => e.stopPropagation()}
      onClick={readOnly ? readOnlyLink.onClick : undefined}
      onDoubleClick={readOnly ? readOnlyLink.onDoubleClick : undefined}
    />
  )
}

function linkUrlAtEventTarget(target: EventTarget | null): string | null {
  if (!(target instanceof Element)) return null
  return target.closest('[data-md-url]')?.getAttribute('data-md-url') ?? null
}

/**
 * Plain click on a link in a read-only editor opens it. The canvas pointer
 * router still owns the pointerdown (select/drag), so this listens to the
 * browser's `click` and filters out the two gestures that share its
 * anatomy: a drag (pointer moved between down and up) and the first click
 * of a double-click (which means "edit", not "open" — the open is deferred
 * one double-click window and cancelled by dblclick).
 */
function useReadOnlyLinkClicks(onOpenLink: ((url: string) => void) | undefined) {
  const pressPosRef = useRef<{ x: number; y: number } | null>(null)
  const openTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  useEffect(
    () => () => {
      if (openTimerRef.current) clearTimeout(openTimerRef.current)
    },
    [],
  )

  return {
    onPointerDown: (event: React.PointerEvent) => {
      pressPosRef.current = { x: event.clientX, y: event.clientY }
    },
    onClick: (event: React.MouseEvent) => {
      if (!onOpenLink || event.detail !== 1) return
      const press = pressPosRef.current
      if (press && Math.hypot(event.clientX - press.x, event.clientY - press.y) > 4) {
        return
      }
      const url = linkUrlAtEventTarget(event.target)
      if (!url) return
      if (openTimerRef.current) clearTimeout(openTimerRef.current)
      openTimerRef.current = setTimeout(() => {
        openTimerRef.current = null
        onOpenLink(url)
      }, 250)
    },
    onDoubleClick: () => {
      if (openTimerRef.current) {
        clearTimeout(openTimerRef.current)
        openTimerRef.current = null
      }
    },
  }
}

function useMarkdownEditor(
  containerRef: React.MutableRefObject<HTMLDivElement | null>,
  props: MarkdownEditorProps,
): void {
  const {
    value,
    onChange,
    onFocus,
    onBlur,
    onEscape,
    onOpenLink,
    isDark,
    autoFocus = false,
    placeholder,
    lineWrap = true,
    selectAllOnAutoFocus = false,
    readOnly = false,
    variant = 'markdown',
    onViewReady,
    onSelectionChange,
  } = props
  const viewRef = useRef<EditorView | null>(null)
  const themeCompartmentRef = useRef<Compartment | null>(null)
  const modeCompartmentRef = useRef<Compartment | null>(null)
  const blurTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const onChangeRef = useRef(onChange)
  const onFocusRef = useRef(onFocus)
  const onBlurRef = useRef(onBlur)
  const onEscapeRef = useRef(onEscape)
  const onOpenLinkRef = useRef(onOpenLink)
  const onSelectionChangeRef = useRef(onSelectionChange)
  onChangeRef.current = onChange
  onFocusRef.current = onFocus
  onBlurRef.current = onBlur
  onEscapeRef.current = onEscape
  onOpenLinkRef.current = onOpenLink
  onSelectionChangeRef.current = onSelectionChange

  // The half of the stack that differs between view and edit mode. Lives in
  // a compartment so the mode can swap without rebuilding the view. Closes
  // over callback refs only, so the mount effect can hold on to one copy.
  const modeExtensions = (mode: boolean): Extension =>
    mode
      ? [EditorView.editable.of(false), EditorState.readOnly.of(true)]
      : [
          EditorView.updateListener.of((update) => {
            if (update.selectionSet || update.docChanged) {
              onSelectionChangeRef.current?.(update.state)
            }
            if (!update.docChanged) return
            if (update.transactions.some((tr) => tr.annotation(externalUpdate))) {
              return
            }
            onChangeRef.current(update.state.doc.toString())
          }),
          EditorView.domEventHandlers({
            focus: () => {
              onFocusRef.current?.()
              return false
            },
            blur: () => {
              // Defer one tick: an Electron WCV layout/focus-reconcile can
              // briefly steal focus from contentDOM and immediately return
              // it. Firing onBlur synchronously would commit on every
              // spurious thrash.
              if (blurTimerRef.current) clearTimeout(blurTimerRef.current)
              blurTimerRef.current = setTimeout(() => {
                blurTimerRef.current = null
                if (viewRef.current?.hasFocus) return
                onBlurRef.current?.()
              }, 0)
              return false
            },
            mousedown: (event) => {
              if (event.metaKey && onOpenLinkRef.current) {
                const url = linkUrlAtEventTarget(event.target)
                if (url) {
                  event.preventDefault()
                  event.stopPropagation()
                  onOpenLinkRef.current(url)
                  return true
                }
              }
              event.stopPropagation()
              return false
            },
            keydown: (event) => {
              if (event.key === 'Escape' && onEscapeRef.current) {
                event.preventDefault()
                onEscapeRef.current()
                return true
              }
              return false
            },
          }),
        ]

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const { extensions, themeCompartment } =
      variant === 'sticky'
        ? createStickyTextExtensions(isDark, { lineWrap })
        : createMarkdownExtensions(isDark, { lineWrap })
    themeCompartmentRef.current = themeCompartment
    const modeCompartment = new Compartment()
    modeCompartmentRef.current = modeCompartment

    const editorExtensions: Extension[] = [
      ...extensions,
      modeCompartment.of(modeExtensions(readOnly)),
    ]
    if (placeholder) editorExtensions.push(placeholderExtension(placeholder))

    const view = new EditorView({
      state: EditorState.create({
        doc: value,
        extensions: editorExtensions,
      }),
      parent: container,
    })
    viewRef.current = view
    onViewReady?.(view)

    return () => {
      if (blurTimerRef.current) {
        clearTimeout(blurTimerRef.current)
        blurTimerRef.current = null
      }
      onViewReady?.(null)
      view.destroy()
      viewRef.current = null
      themeCompartmentRef.current = null
      modeCompartmentRef.current = null
    }
  }, [])

  // Mode swap. The mount effect already applied `readOnly`, so the ref skips
  // this effect's own first run and it reconfigures only on a real change.
  const readOnlyRef = useRef(readOnly)
  useEffect(() => {
    const view = viewRef.current
    const compartment = modeCompartmentRef.current
    if (!view || !compartment) return
    if (readOnlyRef.current === readOnly) return
    readOnlyRef.current = readOnly
    view.dispatch({ effects: compartment.reconfigure(modeExtensions(readOnly)) })
    // Leaving edit mode: drop focus explicitly rather than trusting the
    // browser to blur a now-uneditable contentDOM, so the deferred blur
    // handler still runs and the host commits.
    if (readOnly) view.contentDOM.blur()
  }, [readOnly])

  // Autofocus on mount and on every entry into edit mode. Declared after the
  // mode swap so the view is already editable when focus lands.
  useEffect(() => {
    const view = viewRef.current
    if (!view || !autoFocus || readOnly) return
    view.focus()
    view.dispatch({
      selection: autofocusEditorSelection(view.state.doc.length, selectAllOnAutoFocus),
    })
  }, [readOnly, autoFocus])

  useEffect(() => {
    const view = viewRef.current
    const compartment = themeCompartmentRef.current
    if (!view || !compartment) return
    reconfigureTheme(view, compartment, isDark)
  }, [isDark])

  useEffect(() => {
    const view = viewRef.current
    if (!view) return
    const current = view.state.doc.toString()
    if (current === value) return
    // Apply external updates even while focused so Yjs-driven changes
    // (e.g. cross-stack undo) reflect mid-edit. Cursor resets to end —
    // acceptable for undo, same coarse behavior the textarea predecessor had.
    const insertEnd = value.length
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: value },
      selection: { anchor: insertEnd },
      annotations: externalUpdate.of(true),
    })
  }, [value])

}
