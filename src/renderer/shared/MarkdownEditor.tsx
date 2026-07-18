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
 * The host stops mousedown propagation so the canvas pointer router
 * doesn't treat clicks inside the editor as canvas drags.
 */
export function MarkdownEditor({
  value,
  onChange,
  onFocus,
  onBlur,
  onEscape,
  isDark,
  autoFocus = false,
  placeholder,
  className,
  style,
  lineWrap = true,
  selectAllOnAutoFocus = false,
}: {
  value: string
  onChange: (value: string) => void
  onFocus?: () => void
  onBlur?: () => void
  /** Called when the user presses Escape inside the editor. */
  onEscape?: () => void
  isDark: boolean
  autoFocus?: boolean
  placeholder?: string
  className?: string
  style?: React.CSSProperties
  /** Disable to let the editor's container expand horizontally to fit content. */
  lineWrap?: boolean
  /** Select the full value when auto-focusing; intended for short text nodes. */
  selectAllOnAutoFocus?: boolean
}) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const viewRef = useRef<EditorView | null>(null)
  const themeCompartmentRef = useRef<Compartment | null>(null)
  const blurTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const onChangeRef = useRef(onChange)
  const onFocusRef = useRef(onFocus)
  const onBlurRef = useRef(onBlur)
  const onEscapeRef = useRef(onEscape)
  onChangeRef.current = onChange
  onFocusRef.current = onFocus
  onBlurRef.current = onBlur
  onEscapeRef.current = onEscape

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const { extensions, themeCompartment } = createMarkdownExtensions(isDark, { lineWrap })
    themeCompartmentRef.current = themeCompartment

    const editorExtensions: Extension[] = [
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

  return (
    <div
      ref={containerRef}
      className={className}
      style={style}
      onPointerDown={(e) => e.stopPropagation()}
    />
  )
}
