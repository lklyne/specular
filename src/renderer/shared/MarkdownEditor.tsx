import { useEffect, useRef } from 'react'
import { Compartment, EditorState, type Extension } from '@codemirror/state'
import { EditorView, placeholder as placeholderExtension } from '@codemirror/view'
import {
  createMarkdownExtensions,
  externalUpdate,
  reconfigureTheme,
} from '../canvas-bg/entity-renderers/markdown-codemirror'
import { autofocusEditorSelection } from '../../shared/editor-selection'

/**
 * CodeMirror-based markdown editor. Renders the source as live-styled
 * markdown (heading sizes, bold, links) so edit and view modes share the
 * same visual metrics — see MARKDOWN_TOKENS in markdown-codemirror.ts.
 *
 * `readOnly` renders the same view non-editable, so a text body can be
 * displayed by this component in both view and edit mode. That is the point:
 * a second renderer for view mode (react-markdown) can't reproduce the
 * source's literal line breaks, so mode swaps reflow. Read-only mode also
 * leaves pointer events alone so the canvas router still gets the drag.
 *
 * While editable the host stops mousedown propagation so the canvas pointer
 * router doesn't treat clicks inside the editor as canvas drags.
 *
 * `readOnly` is read at mount only — give the element a `key` that changes
 * with the mode so it remounts.
 */
export function MarkdownEditor({
  value,
  onChange,
  onFocus,
  onBlur,
  onEscape,
  onOpenLink,
  isDark,
  autoFocus = false,
  placeholder,
  className,
  style,
  lineWrap = true,
  selectAllOnAutoFocus = false,
  readOnly = false,
}: {
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
  /** Display the value without a caret, focus, or text selection. */
  readOnly?: boolean
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  useMarkdownEditor({
    containerRef,
    value,
    onChange,
    onFocus,
    onBlur,
    onEscape,
    onOpenLink,
    isDark,
    autoFocus,
    placeholder,
    lineWrap,
    selectAllOnAutoFocus,
    readOnly,
  })
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

interface MarkdownEditorRuntimeOptions {
  containerRef: React.MutableRefObject<HTMLDivElement | null>
  value: string
  onChange: (value: string) => void
  onFocus?: () => void
  onBlur?: () => void
  onEscape?: () => void
  onOpenLink?: (url: string) => void
  isDark: boolean
  autoFocus: boolean
  placeholder?: string
  lineWrap: boolean
  selectAllOnAutoFocus: boolean
  readOnly: boolean
}

function useMarkdownEditor(options: MarkdownEditorRuntimeOptions): void {
  const {
    containerRef,
    value,
    onChange,
    onFocus,
    onBlur,
    onEscape,
    onOpenLink,
    isDark,
    autoFocus,
    placeholder,
    lineWrap,
    selectAllOnAutoFocus,
    readOnly,
  } = options
  const viewRef = useRef<EditorView | null>(null)
  const themeCompartmentRef = useRef<Compartment | null>(null)
  const blurTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const onChangeRef = useRef(onChange)
  const onFocusRef = useRef(onFocus)
  const onBlurRef = useRef(onBlur)
  const onEscapeRef = useRef(onEscape)
  const onOpenLinkRef = useRef(onOpenLink)
  onChangeRef.current = onChange
  onFocusRef.current = onFocus
  onBlurRef.current = onBlur
  onEscapeRef.current = onEscape
  onOpenLinkRef.current = onOpenLink

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const { extensions, themeCompartment } = createMarkdownExtensions(isDark, { lineWrap })
    themeCompartmentRef.current = themeCompartment

    const editorExtensions: Extension[] = readOnly ? [
      ...extensions,
      EditorView.editable.of(false),
      EditorState.readOnly.of(true),
    ] : [
      ...extensions,
      EditorView.updateListener.of((update) => {
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
    if (placeholder) editorExtensions.push(placeholderExtension(placeholder))
    // `readOnly` above is captured at mount; the hook has no deps by design.

    const view = new EditorView({
      state: EditorState.create({
        doc: value,
        extensions: editorExtensions,
      }),
      parent: container,
    })
    viewRef.current = view

    if (autoFocus) {
      view.focus()
      view.dispatch({
        selection: autofocusEditorSelection(
          view.state.doc.length,
          selectAllOnAutoFocus,
        ),
      })
    }

    return () => {
      if (blurTimerRef.current) {
        clearTimeout(blurTimerRef.current)
        blurTimerRef.current = null
      }
      view.destroy()
      viewRef.current = null
      themeCompartmentRef.current = null
    }
  }, [])

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
