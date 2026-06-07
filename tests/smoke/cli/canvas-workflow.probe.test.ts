import { describe, it, expect } from 'vitest'
import { runCli } from './cli-probe-utils'

// Prescribed canvas workflow (see packs/cli/charter.md): an agent builds a small
// canvas with the CLI, then reads it back. The probe asserts the *mechanical
// friction signals* the charter defines — parseable output, zero exit, the edit
// is observable — so a CLI that technically works but is awkward fails here.
//
// Friction these guard against:
//   - output that isn't valid JSON (agent can't parse the result of its own edit)
//   - a successful edit that doesn't show up when reading the workspace back

describe('cli probe: canvas editing workflow', () => {
  it('create page returns parseable JSON and exits zero', () => {
    const r = runCli(['create', 'page', 'https://example.com'])
    expect(r.code, r.stderr).toBe(0)
    // The agent must be able to parse the result of its own edit without a flag.
    expect(r.json, `stdout was not JSON:\n${r.stdout}`).toBeDefined()
  })

  it('the edit is observable when reading the workspace back', () => {
    runCli(['create', 'note', 'probe canvas note'])
    const ws = runCli(['workspace'])
    expect(ws.code, ws.stderr).toBe(0)
    expect(ws.json, `workspace stdout was not JSON:\n${ws.stdout}`).toBeDefined()
    // The note we just created is reachable by reading the canvas — one intent,
    // verifiable in one read.
    expect(ws.stdout).toContain('probe canvas note')
  })

  it('a full breakpoint-style workflow completes without an error exit', () => {
    // Prescribed multi-step task: two pages + a note. Every step must succeed;
    // a non-zero exit anywhere is friction an agent would have to recover from.
    const steps = [
      ['create', 'page', 'https://example.com'],
      ['create', 'page', 'https://example.org'],
      ['create', 'note', 'breakpoint workflow probe'],
    ]
    for (const args of steps) {
      const r = runCli(args)
      expect(r.code, `\`specular ${args.join(' ')}\` → ${r.stderr}`).toBe(0)
    }
  })
})
