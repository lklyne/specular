/**
 * A ```mermaid fence in a note renders as the diagram it describes.
 *
 * Lives in a StateField rather than in `livePreviewPlugin`: CodeMirror only
 * lets a state field supply a decoration that replaces line breaks, and a
 * fence is several lines. The reveal rule is the live preview's: while the
 * cursor sits on any line of the fence the source shows, so the diagram is
 * still editable in place; a read-only editor reveals nothing.
 */

import { syntaxTree } from '@codemirror/language'
import { StateField, type EditorState, type Extension, type Range } from '@codemirror/state'
import { Decoration, EditorView, WidgetType, type DecorationSet } from '@codemirror/view'
import type { SyntaxNodeRef } from '@lezer/common'
import { loadMermaidRenderer, renderMermaidIfLoaded } from '../mermaid/mermaid-loader'
import { svgElementFromString } from '../mermaid/mermaid-svg-dom'
import type { MermaidRender } from '../mermaid/render-mermaid'
import { revealedLines, type LivePreviewOptions } from './markdown-live-preview'

export interface MermaidFence {
  from: number
  to: number
  source: string
}

/** The fence's diagram source when `node` is a ```mermaid block, else null. */
export function mermaidFenceAt(node: SyntaxNodeRef, state: EditorState): MermaidFence | null {
  if (node.name !== 'FencedCode') return null
  let info: string | null = null
  let source = ''
  for (let child = node.node.firstChild; child; child = child.nextSibling) {
    if (child.name === 'CodeInfo') info = state.sliceDoc(child.from, child.to).trim()
    if (child.name === 'CodeText') source = state.sliceDoc(child.from, child.to)
  }
  if (info !== 'mermaid') return null
  if (source.trim() === '') return null
  return { from: node.from, to: node.to, source }
}

/** Whether the cursor sits on any line of the fence. */
function fenceRevealed(fence: MermaidFence, state: EditorState, revealed: Set<number>): boolean {
  const first = state.doc.lineAt(fence.from).number
  const last = state.doc.lineAt(fence.to).number
  for (let n = first; n <= last; n += 1) if (revealed.has(n)) return true
  return false
}

export class MermaidWidget extends WidgetType {
  constructor(readonly source: string) {
    super()
  }
  eq(other: MermaidWidget): boolean {
    return other.source === this.source
  }
  toDOM(): HTMLElement {
    const host = document.createElement('div')
    host.className = 'cm-md-mermaid'
    const paint = (result: MermaidRender) => {
      const el = result.ok ? svgElementFromString(result.svg) : null
      if (el) {
        host.replaceChildren(el)
        return
      }
      const fallback = document.createElement('pre')
      fallback.className = 'cm-md-mermaid-error'
      fallback.textContent = result.ok ? this.source : `${this.source}\n\n${result.error}`
      host.replaceChildren(fallback)
    }
    const sync = renderMermaidIfLoaded(this.source)
    if (sync) paint(sync)
    else {
      loadMermaidRenderer().then((mod) => {
        if (host.isConnected) paint(mod.renderMermaidCached(this.source))
      })
    }
    return host
  }
  // A click lands the cursor at the fence, which reveals the source.
  ignoreEvent(): boolean {
    return false
  }
}

/** Pure over `state`, like `buildMarkdownDecorations`, so it runs without a DOM. */
export function buildMermaidDecorations(
  state: EditorState,
  options: LivePreviewOptions,
): DecorationSet {
  if (!options.diagrams) return Decoration.none
  const decorations: Range<Decoration>[] = []
  const revealed = revealedLines(state, options)
  syntaxTree(state).iterate({
    enter: (node) => {
      const fence = mermaidFenceAt(node, state)
      if (!fence) return undefined
      if (!fenceRevealed(fence, state, revealed)) {
        decorations.push(
          Decoration.replace({ widget: new MermaidWidget(fence.source), block: true }).range(
            fence.from,
            fence.to,
          ),
        )
      }
      return false
    },
  })
  return Decoration.set(decorations, true)
}

function createMermaidField(options: LivePreviewOptions) {
  return StateField.define<DecorationSet>({
    create: (state) => buildMermaidDecorations(state, options),
    update: (decorations, tr) => {
      // The parser advances in the background and announces itself with a
      // new tree on an otherwise empty transaction, so tree identity is
      // checked alongside the document and the cursor.
      const treeChanged = syntaxTree(tr.state) !== syntaxTree(tr.startState)
      if (!tr.docChanged && !tr.selection && !tr.reconfigured && !treeChanged) return decorations
      return buildMermaidDecorations(tr.state, options)
    },
    provide: (field) => [
      EditorView.decorations.from(field),
      EditorView.atomicRanges.of((view) => view.state.field(field)),
    ],
  })
}

const mermaidTheme = EditorView.theme({
  '.cm-md-mermaid': { padding: '0.5em 0' },
  '.cm-md-mermaid svg': { maxWidth: '100%', height: 'auto', display: 'block' },
  '.cm-md-mermaid-error': {
    margin: '0',
    whiteSpace: 'pre-wrap',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
    fontSize: '0.85em',
    opacity: '0.7',
  },
})

export function markdownMermaidDiagrams(options: LivePreviewOptions): Extension {
  return [createMermaidField(options), mermaidTheme]
}
