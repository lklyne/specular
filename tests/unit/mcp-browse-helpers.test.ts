import { describe, it, expect } from 'vitest'
import {
  splitShellArgs,
  shellQuote,
  splitChainedCommands,
  parseCommandArgs,
  staleGenerationWarning,
} from '../../src/main/mcp-browse'

describe('splitShellArgs', () => {
  it('splits simple space-separated args', () => {
    expect(splitShellArgs('click @e5')).toEqual(['click', '@e5'])
  })

  it('handles double-quoted strings', () => {
    expect(splitShellArgs('fill @e3 "hello world"')).toEqual([
      'fill', '@e3', 'hello world',
    ])
  })

  it('handles single-quoted strings', () => {
    expect(splitShellArgs("snapshot -s '#main'")).toEqual([
      'snapshot', '-s', '#main',
    ])
  })

  it('handles escaped characters', () => {
    expect(splitShellArgs('fill @e1 hello\\ world')).toEqual([
      'fill', '@e1', 'hello world',
    ])
  })

  it('handles empty input', () => {
    expect(splitShellArgs('')).toEqual([])
    expect(splitShellArgs('   ')).toEqual([])
  })

  it('handles multiple whitespace between args', () => {
    expect(splitShellArgs('click   @e5')).toEqual(['click', '@e5'])
  })

  it('handles nested quotes', () => {
    expect(splitShellArgs('fill @e1 "it\'s here"')).toEqual([
      'fill', '@e1', "it's here",
    ])
  })
})

describe('splitChainedCommands', () => {
  it('returns a single element when no unquoted && present', () => {
    expect(splitChainedCommands('eval document.title')).toEqual(['eval document.title'])
  })

  it('splits on unquoted &&', () => {
    expect(splitChainedCommands('click @e1 && wait --load networkidle && snapshot -i')).toEqual([
      'click @e1',
      'wait --load networkidle',
      'snapshot -i',
    ])
  })

  it('does not split on && inside double quotes', () => {
    expect(splitChainedCommands('eval "a && b"')).toEqual(['eval "a && b"'])
  })

  it('does not split on && inside single quotes', () => {
    expect(splitChainedCommands("eval 'a && b && c'")).toEqual(["eval 'a && b && c'"])
  })

  it('splits mixed: unquoted chain with a quoted && inside one part', () => {
    expect(splitChainedCommands(`eval 'x && y' && snapshot -i`)).toEqual([
      "eval 'x && y'",
      'snapshot -i',
    ])
  })
})

describe('shellQuote + splitShellArgs round-trip', () => {
  const cases: Array<{ label: string; argv: string[] }> = [
    { label: 'bare tokens pass through', argv: ['eval', 'document.title'] },
    { label: 'js string with double quotes', argv: ['eval', 'document.querySelectorAll("iframe").length'] },
    { label: 'js with wildcards and spaces', argv: ['eval', 'Array.from(document.querySelectorAll("*")).filter(el => el.textContent === "Current Location")'] },
    { label: 'single quote in content', argv: ['eval', "document.title + 'x'"] },
    { label: 'mixed quotes + spaces', argv: ['eval', `el.getAttribute("data-foo") === 'bar baz'`] },
    { label: 'selector with special chars', argv: ['get', 'styles', 'div.css-text-146c3p1 > span'] },
  ]
  for (const { label, argv } of cases) {
    it(label, () => {
      const joined = argv.map(shellQuote).join(' ')
      expect(splitShellArgs(joined)).toEqual(argv)
    })
  }
})

describe('parseCommandArgs', () => {
  it('extracts verb from simple command', () => {
    const result = parseCommandArgs('snapshot')
    expect(result.verb).toBe('snapshot')
    expect(result.ref).toBeNull()
  })

  it('extracts verb and ref', () => {
    const result = parseCommandArgs('click @e5')
    expect(result.verb).toBe('click')
    expect(result.ref).toBe('@e5')
  })

  it('skips flags with values', () => {
    const result = parseCommandArgs('snapshot -s "#main" --depth 3')
    expect(result.verb).toBe('snapshot')
    expect(result.ref).toBeNull()
  })

  it('handles fill command with ref and text', () => {
    const result = parseCommandArgs('fill @e12 hello')
    expect(result.verb).toBe('fill')
    expect(result.ref).toBe('@e12')
  })

  it('returns null verb for empty command', () => {
    const result = parseCommandArgs('')
    expect(result.verb).toBeNull()
    expect(result.ref).toBeNull()
  })

  it('skips boolean flags', () => {
    const result = parseCommandArgs('snapshot -i')
    expect(result.verb).toBe('snapshot')
    expect(result.ref).toBeNull()
  })

  it('handles CDP flag before verb', () => {
    const result = parseCommandArgs('--cdp ws://localhost:9222 click @e3')
    expect(result.verb).toBe('click')
    expect(result.ref).toBe('@e3')
  })
})

// D8 (issue #318): generation-based staleness detection is warn-only. This
// is the pure comparison at the heart of it — everything else in
// browse-handler.ts (caching, fresh fetch, prepending) is plumbing around
// this one `>` check.
describe('staleGenerationWarning', () => {
  it('returns null when the page has not navigated since the snapshot', () => {
    expect(staleGenerationWarning('page_1', 3, 3)).toBeNull()
  })

  it('returns null when the current generation is somehow behind (never regresses into a warning)', () => {
    expect(staleGenerationWarning('page_1', 5, 2)).toBeNull()
  })

  it('warns with the page id and recovery guidance when the generation has moved on', () => {
    const warning = staleGenerationWarning('page_42', 1, 2)
    expect(warning).toContain('page changed since your last snapshot')
    expect(warning).toContain('refs likely stale')
    expect(warning).toContain('specular snapshot -i -f page_42')
    expect(warning).toContain('text=/CSS selector')
  })
})
