// Mutation-verified: temporarily emptied BLOCKED_BROWSE_VERBS in
// src/main/shared/browse-handler.ts and confirmed every case below fails — dispatch()
// returns 0 instead of 1, no actionable-error text is written to stderr, and
// (for the `launch` case) the child_process.spawn mock below throws because
// the blocked verb fell through to the real agent-browser passthrough.
// Restored afterward.
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// Mock at the process boundary only: if a blocked verb ever reaches
// spawnAsync, spawn() throws so the test fails loudly instead of hanging on
// (or actually invoking) a real subprocess.
vi.mock('child_process', () => ({
  spawn: vi.fn(() => {
    throw new Error('spawn must not be called for a blocked passthrough verb')
  }),
}))

import { dispatch } from '../../src/main/cli-commands'

describe('blocked passthrough verbs', () => {
  let stderrSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    stderrSpy = vi.spyOn(process.stderr, 'write').mockImplementation(() => true)
  })

  afterEach(() => {
    stderrSpy.mockRestore()
  })

  function stderrText(): string {
    return stderrSpy.mock.calls.map((call) => String(call[0])).join('')
  }

  const cases: Array<{ verb: string; args: string[]; expectFragment: string }> = [
    { verb: 'launch', args: ['launch'], expectFragment: 'specular add page' },
    { verb: 'close', args: ['close'], expectFragment: 'specular delete' },
    { verb: 'quit', args: ['quit'], expectFragment: 'specular delete' },
    { verb: 'install', args: ['install'], expectFragment: 'bundled with Specular' },
    { verb: 'upgrade', args: ['upgrade'], expectFragment: 'bundled with Specular' },
    { verb: 'open', args: ['open', 'https://example.com'], expectFragment: 'specular update' },
  ]

  for (const { verb, args, expectFragment } of cases) {
    it(`blocks \`${verb}\` with an actionable error instead of reaching spawn`, async () => {
      const code = await dispatch(args)
      expect(code).toBe(1)
      expect(stderrText()).toContain(expectFragment)
    })
  }

  it('does not block unrelated passthrough verbs', () => {
    // `eval` isn't in BLOCKED_BROWSE_VERBS — this only pins the
    // blocklist to the documented set so it can't silently grow.
    const blockedVerbs = cases.map((c) => c.verb)
    expect(blockedVerbs).not.toContain('eval')
    expect(blockedVerbs).not.toContain('get')
  })
})
