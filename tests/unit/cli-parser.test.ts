import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, it, expect } from 'vitest'
import { parseArgs, CLI_VALUE_FLAGS } from '../../src/main/cli-parser'
import { buildSnapshotCommand } from '../../src/main/cli-commands'

describe('parseArgs', () => {
  it('extracts verb from simple command', () => {
    const result = parseArgs(['workspace'])
    expect(result.verb).toBe('workspace')
    expect(result.positional).toEqual([])
  })

  it('extracts verb and positional args', () => {
    const result = parseArgs(['click', '@e5'])
    expect(result.verb).toBe('click')
    expect(result.positional).toEqual(['@e5'])
  })

  it('extracts named flags', () => {
    const result = parseArgs(['snapshot', '--page', 'abc123'])
    expect(result.verb).toBe('snapshot')
    expect(result.flags.page).toBe('abc123')
  })

  it('extracts boolean flags', () => {
    const result = parseArgs(['snapshot', '-i'])
    expect(result.verb).toBe('snapshot')
    expect(result.boolFlags.has('i')).toBe(true)
  })

  it('handles mixed flags and positionals', () => {
    const result = parseArgs(['create', 'page', 'https://example.com', '--preset', '7', '--landscape'])
    expect(result.verb).toBe('create')
    expect(result.positional).toEqual(['page', 'https://example.com'])
    expect(result.flags.preset).toBe('7')
    expect(result.boolFlags.has('landscape')).toBe(true)
  })

  it('handles --page shorthand -f', () => {
    const result = parseArgs(['snapshot', '-f', 'page-123', '-i'])
    expect(result.flags.f).toBe('page-123')
    expect(result.boolFlags.has('i')).toBe(true)
  })

  it('handles empty input', () => {
    const result = parseArgs([])
    expect(result.verb).toBe('')
    expect(result.positional).toEqual([])
  })

  it('handles -- separator', () => {
    const result = parseArgs(['fill', '@e3', '--', '--not-a-flag'])
    expect(result.verb).toBe('fill')
    expect(result.positional).toEqual(['@e3', '--not-a-flag'])
  })

  it('handles multiple positionals for fill', () => {
    const result = parseArgs(['fill', '@e12', 'hello', 'world'])
    expect(result.verb).toBe('fill')
    expect(result.positional).toEqual(['@e12', 'hello', 'world'])
  })

  it('handles annotation filter flags', () => {
    const result = parseArgs(['annotations', '--status', 'pending', '--url', 'https://example.com'])
    expect(result.verb).toBe('annotations')
    expect(result.flags.status).toBe('pending')
    expect(result.flags.url).toBe('https://example.com')
  })

  it('handles update with --at coordinates', () => {
    const result = parseArgs(['update', 'page-123', '--at', '800,400', '--preset', '3'])
    expect(result.verb).toBe('update')
    expect(result.positional).toEqual(['page-123'])
    expect(result.flags.at).toBe('800,400')
    expect(result.flags.preset).toBe('3')
  })

  it('treats --size and --cols as value flags, not booleans', () => {
    // Regression: both were missing from CLI_VALUE_FLAGS, so their values
    // leaked into positionals (note --size silently dropped; arrange --cols
    // read the count as an entity id).
    const size = parseArgs(['update', 'text-1', '--size', '360,240'])
    expect(size.flags.size).toBe('360,240')
    expect(size.positional).toEqual(['text-1'])

    const cols = parseArgs(['arrange', 'grid', 'a', 'b', '--cols', '2'])
    expect(cols.flags.cols).toBe('2')
    expect(cols.positional).toEqual(['grid', 'a', 'b'])
  })

  it('preserves rest for passthrough', () => {
    const result = parseArgs(['eval', 'document.title'])
    expect(result.verb).toBe('eval')
    expect(result.rest).toEqual(['document.title'])
  })

  it('handles record subcommands', () => {
    const result = parseArgs(['record', 'start', 'page-123', '--output', '/tmp/video.webm'])
    expect(result.verb).toBe('record')
    expect(result.positional).toEqual(['start', 'page-123'])
    expect(result.flags.output).toBe('/tmp/video.webm')
  })

  it('handles help flag', () => {
    const result = parseArgs(['--help'])
    expect(result.boolFlags.has('help')).toBe(true)
    expect(result.verb).toBe('')
  })

  it('preserves agent-browser snapshot context reducers', () => {
    const result = parseArgs([
      'snapshot',
      '-i',
      '--compact',
      '--urls',
      '-s',
      '#External_links',
      '-d',
      '3',
    ])
    expect(buildSnapshotCommand(result)).toBe("snapshot -i -c -u -s '#External_links' -d 3")
  })
})

// Drift guard: every `args.flags.X` a verb handler reads must be a declared
// value flag, else the parser treats `--X` as boolean and its value leaks to
// positionals — a silent no-op (e.g. `--size`, `--cols` once did). This keeps
// the consumer list and the parser's allowlist from diverging again.
describe('CLI_VALUE_FLAGS covers every consumed flag', () => {
  it('has a value-flag entry for each args.flags.X read in cli-commands.ts', () => {
    const src = readFileSync(
      fileURLToPath(new URL('../../src/main/cli-commands.ts', import.meta.url)),
      'utf8',
    )
    const consumed = new Set(
      [...src.matchAll(/args\.flags\.([a-zA-Z]+)/g)].map((m) => m[1]),
    )
    // Parser stores flags dash-stripped, so a key matches either --key or -k.
    const declared = new Set(
      [...CLI_VALUE_FLAGS].map((f) => f.replace(/^-+/, '')),
    )
    const missing = [...consumed].filter((k) => !declared.has(k))
    expect(missing).toEqual([])
  })
})
