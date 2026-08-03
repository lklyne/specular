/**
 * Placeholder for an empty editor, drawn as generated content on the empty
 * line instead of CodeMirror's own `placeholder()` extension.
 *
 * CodeMirror's version inserts a widget — a real `contenteditable="false"`
 * inline-block element — at the one position the caret can occupy in an empty
 * document, and Chromium paints no caret next to it. A brand-new sticky
 * therefore reads as unfocused until the first keystroke clears the widget.
 *
 * A line decoration leaves the line genuinely empty, so the caret sits in the
 * line box like it does on any other empty line, and the prompt is painted
 * over it out of flow.
 */

import type { Extension } from '@codemirror/state'
import { Decoration, type DecorationSet, EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view'

const placeholderTheme = EditorView.baseTheme({
  '.cm-line[data-placeholder]': { position: 'relative' },
  '.cm-line[data-placeholder]::before': {
    content: 'attr(data-placeholder)',
    position: 'absolute',
    left: '0',
    top: '0',
    color: '#888',
    pointerEvents: 'none',
    userSelect: 'none',
  },
})

export function placeholderExtension(text: string): Extension {
  const lineDecoration = Decoration.line({ attributes: { 'data-placeholder': text } })
  const decorate = (view: EditorView): DecorationSet =>
    view.state.doc.length ? Decoration.none : Decoration.set([lineDecoration.range(0)])

  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet
      constructor(view: EditorView) {
        this.decorations = decorate(view)
      }
      update(update: ViewUpdate) {
        if (update.docChanged) this.decorations = decorate(update.view)
      }
    },
    { decorations: (v) => v.decorations },
  )

  return [plugin, placeholderTheme, EditorView.contentAttributes.of({ 'aria-placeholder': text })]
}
