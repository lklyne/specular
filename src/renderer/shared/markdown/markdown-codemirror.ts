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

// fontSize and lineHeight are inherited from the editor's wrapper so
// per-entity `textSize` and its size-scaled leading (ADR 0013 §2) flow
// through to CodeMirror; heading sizes stay relative via `em`.
const MARKDOWN_TOKENS = {
  fontFamily: 'system-ui, sans-serif',
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

function buildEditorTheme(isDark: boolean): Extension {
  return EditorView.theme(
    {
      '&': {
        backgroundColor: 'transparent',
        color: isDark ? '#e7e5e4' : '#1c1917',
        fontSize: 'inherit',
        fontFamily: MARKDOWN_TOKENS.fontFamily,
        height: '100%',
      },
      '.cm-content': {
        padding: '0',
        caretColor: isDark ? '#e7e5e4' : '#1c1917',
      },
      '.cm-line': { padding: '0' },
      '&.cm-focused': { outline: 'none' },
      '.cm-scroller': {
        overflow: 'auto',
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
) {
  view.dispatch({ effects: compartment.reconfigure(buildEditorTheme(isDark)) })
}

export function createMarkdownExtensions(
  isDark: boolean,
  options: { lineWrap?: boolean } = {},
): {
  extensions: Extension[]
  themeCompartment: Compartment
} {
  const themeCompartment = new Compartment()
  const lineWrap = options.lineWrap ?? true
  const extensions: Extension[] = [
    // No `history()` extension: undo/redo is owned by the Yjs UndoManager
    // in main, not by per-editor CodeMirror history. Cmd+Z falls through
    // to the canvas keyboard handler so text and canvas edits share one
    // unified undo stack.
    // Cmd+B / Cmd+I / Cmd+K etc.
    keymap.of(markdownFormattingKeymap),
    // Enter continues the current list/quote markup, Backspace peels one level
    // off. Bound here rather than via markdown()'s `addKeymap` so it outranks
    // defaultKeymap's plain-newline Enter.
    keymap.of(markdownKeymap),
    keymap.of(defaultKeymap),
    // GFM base, so strikethrough and task lists parse — the highlight style
    // and live preview below both style them.
    markdown({ base: markdownLanguage, addKeymap: false }),
    syntaxHighlighting(markdownHighlightStyle),
    markdownLivePreview(FULL_LIVE_PREVIEW),
    smartPasteExtension(),
  ]
  // When wrap is off (auto-width plain text), the editor's container shrinks
  // to fit each line's natural width instead of forcing single-character wrap.
  if (lineWrap) extensions.push(EditorView.lineWrapping)
  extensions.push(themeCompartment.of(buildEditorTheme(isDark)))
  return { extensions, themeCompartment }
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
  options: { lineWrap?: boolean } = {},
): {
  extensions: Extension[]
  themeCompartment: Compartment
} {
  const themeCompartment = new Compartment()
  const lineWrap = options.lineWrap ?? true
  const extensions: Extension[] = [
    // Order matters: each keymap's Backspace/Enter binding falls through
    // (returns false) to the next when it doesn't apply, so mark-unwrap
    // outranks bullet-prefix strip outranks plain formatting outranks
    // CodeMirror's default character deletion / newline.
    keymap.of(stickyDeleteKeymap),
    keymap.of(stickyListKeymap),
    keymap.of(stickyFormattingKeymap),
    keymap.of(defaultKeymap),
    markdown({ base: markdownLanguage, addKeymap: false }),
    syntaxHighlighting(stickyHighlightStyle),
    markdownLivePreview(STICKY_LIVE_PREVIEW),
  ]
  if (lineWrap) extensions.push(EditorView.lineWrapping)
  extensions.push(themeCompartment.of(buildEditorTheme(isDark)))
  return { extensions, themeCompartment }
}
