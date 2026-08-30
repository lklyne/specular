import { Annotation, Compartment, type Extension } from '@codemirror/state'
import { EditorView, keymap } from '@codemirror/view'
import { defaultKeymap } from '@codemirror/commands'
import { markdown, markdownKeymap, markdownLanguage } from '@codemirror/lang-markdown'
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { tags as t } from '@lezer/highlight'
import { smartPasteExtension } from './markdown-smart-paste'
import {
  FULL_LIVE_PREVIEW,
  STICKY_LIVE_PREVIEW,
  markdownLivePreview,
} from './markdown-live-preview'
import { markdownFormattingKeymap, stickyFormattingKeymap } from './markdown-commands'
import { stickyDeleteKeymap, stickyListKeymap } from './sticky-editing'

export const externalUpdate = Annotation.define<boolean>()

// fontSize, lineHeight, and fontFamily are inherited from the editor's wrapper
// so per-entity `textSize`, its size-scaled leading (ADR 0013 §2), and
// `textFont` all flow through to CodeMirror; heading sizes stay relative via
// `em`, so they scale with whatever the wrapper sets.
const MARKDOWN_TOKENS = {
  headingWeight: '600',
  h1Size: '1.4em',
  h2Size: '1.2em',
  h3Size: '1.1em',
  linkColor: '#2563eb',
} as const

const markdownHighlightStyle = HighlightStyle.define([
  { tag: t.heading1, fontWeight: MARKDOWN_TOKENS.headingWeight, fontSize: MARKDOWN_TOKENS.h1Size },
  { tag: t.heading2, fontWeight: MARKDOWN_TOKENS.headingWeight, fontSize: MARKDOWN_TOKENS.h2Size },
  { tag: t.heading3, fontWeight: MARKDOWN_TOKENS.headingWeight, fontSize: MARKDOWN_TOKENS.h3Size },
  { tag: t.heading4, fontWeight: MARKDOWN_TOKENS.headingWeight },
  { tag: t.heading5, fontWeight: MARKDOWN_TOKENS.headingWeight },
  { tag: t.heading6, fontWeight: MARKDOWN_TOKENS.headingWeight },
  { tag: t.strong, fontWeight: MARKDOWN_TOKENS.headingWeight },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.strikethrough, textDecoration: 'line-through' },
  { tag: t.link, textDecoration: 'underline', color: MARKDOWN_TOKENS.linkColor },
  { tag: t.url, color: MARKDOWN_TOKENS.linkColor },
  {
    tag: t.monospace,
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
  },
  // Markup punctuation is collapsed entirely by markdownLivePreview except on
  // the cursor's line; these opacities are how it looks when revealed there.
  { tag: t.processingInstruction, opacity: '0.45' },
  { tag: t.contentSeparator, opacity: '0.45' },
  { tag: t.quote, fontStyle: 'italic' },
])

// Sticky notes only ever render the reduced subset of markdown that stays
// live — bold/italic/strikethrough. No heading sizes, no monospace, no link
// colors, no dimmed-punctuation styling: everything else (`#`, `>`,
// backticks, URLs) is literal text `markdownLivePreview(STICKY_LIVE_PREVIEW)`
// never collapses, so it must read as plain body copy, not styled markup.
const stickyHighlightStyle = HighlightStyle.define([
  { tag: t.strong, fontWeight: MARKDOWN_TOKENS.headingWeight },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.strikethrough, textDecoration: 'line-through' },
])

/**
 * Reading padding belongs on `.cm-content`, inside the scroller, not on a
 * wrapper around the editor. A padded wrapper leaves the scroller inset from
 * the card: the scrollbar floats off the right edge, and scrolled text slides
 * under the top padding and clips there instead of scrolling out of a gutter
 * that travels with it. Padding on the content is part of what scrolls, so the
 * scroller can span the full card.
 *
 * Surfaces that pad outside the editor (sticky notes size their shell to the
 * measured content column) pass '0' and keep their own layout.
 */
function buildEditorTheme(isDark: boolean, contentPadding: string): Extension {
  return EditorView.theme(
    {
      '&': {
        backgroundColor: 'transparent',
        // Inherited for the same reason as the typeface below: the surface
        // wrapping the editor owns the ink (plain text paints its own color
        // slot), and a color set here would win over it.
        color: 'inherit',
        fontSize: 'inherit',
        // Inherited, not fixed: the surface wrapping the editor owns the
        // typeface (a sticky's is per-entity), and a face set here would win
        // over it — so text would reflow the moment editing began.
        fontFamily: 'inherit',
        height: '100%',
      },
      '.cm-content': {
        padding: contentPadding,
        // Follows the ink, so the caret stays visible whatever color the
        // surface is painting in.
        caretColor: 'currentColor',
      },
      '.cm-line': { padding: '0' },
      '&.cm-focused': { outline: 'none' },
      '.cm-scroller': {
        // Canvas text never scrolls sideways: an auto-width entity is sized by
        // its longest line and a fixed-width one wraps, so a horizontal bar
        // only ever means something has mismeasured. `hidden` rather than
        // `clip` so CodeMirror can still scroll the caret back into view while
        // a word too long to break is being typed — no bar, nothing stranded.
        overflowX: 'hidden',
        overflowY: 'auto',
        fontFamily: 'inherit',
        lineHeight: 'inherit',
      },
      '.cm-gutters': { display: 'none' },
    },
    { dark: isDark },
  )
}

export function reconfigureTheme(
  view: EditorView,
  compartment: Compartment,
  isDark: boolean,
  contentPadding: string,
) {
  view.dispatch({
    effects: compartment.reconfigure(buildEditorTheme(isDark, contentPadding)),
  })
}

/** GFM base, so strikethrough and task lists parse — each variant's
 *  highlight style and live preview both style them. */
function markdownLanguageExtension(): Extension {
  return markdown({ base: markdownLanguage, addKeymap: false })
}

type EditorStackOptions = { lineWrap?: boolean; contentPadding: string }

/**
 * Shared assembly tail for both editor variants: the variant's stack
 * (keymaps, language, styling), then optional line wrapping and the theme
 * compartment. Neither variant mounts `history()`: undo/redo is owned by
 * the Yjs UndoManager in main, so Cmd+Z falls through to the canvas
 * keyboard handler and text and canvas edits share one unified undo stack.
 */
function assembleEditorStack(
  options: EditorStackOptions & { isDark: boolean; stack: Extension[] },
): {
  extensions: Extension[]
  themeCompartment: Compartment
} {
  const themeCompartment = new Compartment()
  const extensions = [...options.stack]
  // When wrap is off (auto-width plain text), the editor's container shrinks
  // to fit each line's natural width instead of forcing single-character wrap.
  if (options.lineWrap ?? true) extensions.push(EditorView.lineWrapping)
  extensions.push(themeCompartment.of(buildEditorTheme(options.isDark, options.contentPadding)))
  return { extensions, themeCompartment }
}

export function createMarkdownExtensions(
  isDark: boolean,
  options: EditorStackOptions,
): {
  extensions: Extension[]
  themeCompartment: Compartment
} {
  return assembleEditorStack({
    ...options,
    isDark,
    stack: [
      // Cmd+B / Cmd+I / Cmd+K etc.
      keymap.of(markdownFormattingKeymap),
      // Enter continues the current list/quote markup, Backspace peels one
      // level off. Bound here rather than via markdown()'s `addKeymap` so it
      // outranks defaultKeymap's plain-newline Enter.
      keymap.of(markdownKeymap),
      keymap.of(defaultKeymap),
      markdownLanguageExtension(),
      syntaxHighlighting(markdownHighlightStyle),
      markdownLivePreview(FULL_LIVE_PREVIEW),
      smartPasteExtension(),
    ],
  })
}

/**
 * The reduced CodeMirror stack for sticky notes: bold, italic, strikethrough,
 * and bullet lists stay live; everything else markdown supports (headings,
 * quotes, tasks, rules, link chrome) renders as literal text. Marks for the
 * supported features are always hidden — see STICKY_LIVE_PREVIEW — so there
 * is no `markdownKeymap` (it auto-continues quotes/ordered lists, which
 * don't apply here) and no per-editor `history()` (Yjs's UndoManager owns
 * undo, same as the full editor).
 */
export function createStickyTextExtensions(
  isDark: boolean,
  options: EditorStackOptions,
): {
  extensions: Extension[]
  themeCompartment: Compartment
} {
  return assembleEditorStack({
    ...options,
    isDark,
    stack: [
      // Order matters: each keymap's Backspace/Enter binding falls through
      // (returns false) to the next when it doesn't apply, so mark-unwrap
      // outranks bullet-prefix strip outranks plain formatting outranks
      // CodeMirror's default character deletion / newline.
      keymap.of(stickyDeleteKeymap),
      keymap.of(stickyListKeymap),
      keymap.of(stickyFormattingKeymap),
      keymap.of(defaultKeymap),
      markdownLanguageExtension(),
      syntaxHighlighting(stickyHighlightStyle),
      markdownLivePreview(STICKY_LIVE_PREVIEW),
    ],
  })
}
