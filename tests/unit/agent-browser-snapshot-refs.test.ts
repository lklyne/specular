import { describe, it, expect } from 'vitest'
import { parseAgentBrowserSnapshotRefs } from '../../src/main/shared/browse-handler'

// Real output captured from the pinned binary (resources/bin/agent-browser)
// via `agent-browser --content-boundaries --max-output 100000 --session X
// snapshot -i` — exactly what handleBrowse receives on stdout. Kept verbatim
// as a fixture rather than hand-assembled, since the whole point of this
// parser is to survive the format's real quirks (escaped quotes, a name that
// itself contains brackets, trailing-whitespace names, a nameless combobox).
const INTERACTIVE_SNAPSHOT = [
  '--- AGENT_BROWSER_PAGE_CONTENT nonce=176a8aae04777b8d0e8436b1d834fcf9 origin=http://127.0.0.1:38917/ ---',
  '- heading "Log in" [level=1, ref=e1]',
  '- textbox "Email " [ref=e9]',
  '- textbox "Password " [ref=e10]',
  '- button "Sign in" [ref=e5]',
  '- link "Docs" [ref=e6]',
  '- link "Docs" [ref=e7]',
  '- link "Pricing" [ref=e8]',
  '- button "Close dialog" [ref=e2]',
  '- combobox [expanded=false, ref=e3]: Free',
  '  - option "Free" [selected, ref=e11]',
  '  - option "Pro" [ref=e12]',
  '- checkbox "Accept \\"terms\\" [v2]" [checked=false, ref=e4]',
  '--- END_AGENT_BROWSER_PAGE_CONTENT nonce=176a8aae04777b8d0e8436b1d834fcf9 ---',
].join('\n')

// The non-interactive (no `-i`) form: deeper nesting and ref-less structural
// lines (generic, navigation, LabelText, StaticText) interleaved with the
// same `ref=eN` leaf nodes.
const FULL_SNAPSHOT = [
  '- generic',
  '  - LabelText',
  '    - StaticText "Email "',
  '    - textbox "Email " [ref=e9]',
  '- navigation',
  '  - link "Docs" [ref=e6]',
  '- button "Close dialog" [ref=e2]',
  '  - StaticText "x"',
].join('\n')

describe('parseAgentBrowserSnapshotRefs', () => {
  it('extracts ref, role, and trimmed name from the -i sample', () => {
    expect(parseAgentBrowserSnapshotRefs(INTERACTIVE_SNAPSHOT)).toEqual([
      { ref: 'e1', role: 'heading', name: 'Log in' },
      { ref: 'e9', role: 'textbox', name: 'Email' },
      { ref: 'e10', role: 'textbox', name: 'Password' },
      { ref: 'e5', role: 'button', name: 'Sign in' },
      { ref: 'e6', role: 'link', name: 'Docs' },
      { ref: 'e7', role: 'link', name: 'Docs' },
      { ref: 'e8', role: 'link', name: 'Pricing' },
      { ref: 'e2', role: 'button', name: 'Close dialog' },
      { ref: 'e3', role: 'combobox', name: null },
      { ref: 'e11', role: 'option', name: 'Free' },
      { ref: 'e12', role: 'option', name: 'Pro' },
      { ref: 'e4', role: 'checkbox', name: 'Accept "terms" [v2]' },
    ])
  })

  it('unescapes a quoted name containing literal brackets and escaped quotes', () => {
    const [checkbox] = parseAgentBrowserSnapshotRefs(
      '- checkbox "Accept \\"terms\\" [v2]" [checked=false, ref=e4]',
    )
    expect(checkbox.name).toBe('Accept "terms" [v2]')
  })

  it('skips structural/ref-less lines in the non-interactive tree', () => {
    expect(parseAgentBrowserSnapshotRefs(FULL_SNAPSHOT)).toEqual([
      { ref: 'e9', role: 'textbox', name: 'Email' },
      { ref: 'e6', role: 'link', name: 'Docs' },
      { ref: 'e2', role: 'button', name: 'Close dialog' },
    ])
  })

  it('returns an empty list when nothing has a ref', () => {
    expect(parseAgentBrowserSnapshotRefs('- generic\n  - StaticText "hello"')).toEqual([])
  })

  it('ignores the wrapper banner lines', () => {
    const refs = parseAgentBrowserSnapshotRefs(
      '--- AGENT_BROWSER_PAGE_CONTENT nonce=abc origin=http://x/ ---\n' +
        '- button "Go" [ref=e1]\n' +
        '--- END_AGENT_BROWSER_PAGE_CONTENT nonce=abc ---',
    )
    expect(refs).toEqual([{ ref: 'e1', role: 'button', name: 'Go' }])
  })
})
