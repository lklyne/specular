import { describe, it, expect } from 'vitest'
import { runCli } from './cli-probe-utils'

// Error-path ergonomics (see packs/cli/charter.md friction signals). When an
// agent misuses the CLI, the failure must be machine-legible: a non-zero exit
// and an actionable message on stderr that says what to do, not just what broke.
// These currently pass — they are regression guards that keep the error surface
// agent-friendly as the CLI evolves.

describe('cli probe: error ergonomics', () => {
  it('create page with no url fails non-zero with an actionable usage message', () => {
    const r = runCli(['create', 'page'])
    expect(r.code).not.toBe(0)
    // Actionable: the message names the command and the missing argument.
    expect(r.stderr).toMatch(/usage: specular create page <url>/)
  })

  it('create note with no text fails non-zero with an actionable usage message', () => {
    const r = runCli(['create', 'note'])
    expect(r.code).not.toBe(0)
    expect(r.stderr).toMatch(/usage: specular create note/)
  })

  it('errors go to stderr, never polluting stdout an agent is parsing', () => {
    const r = runCli(['create', 'page'])
    // An agent piping stdout into JSON.parse must not receive the error text.
    expect(r.stdout).toBe('')
    expect(r.stderr.length).toBeGreaterThan(0)
  })
})
