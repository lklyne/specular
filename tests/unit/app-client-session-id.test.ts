import { describe, expect, it } from 'vitest'
import { findAgentHostAncestorId, type ParentPidReader } from '../../src/main/shared/app-client'

// Issue #319 Phase 4: sessionId must group all Bash/MCP subprocesses spawned
// within one agent conversation into a single presence cursor, while two
// concurrent conversations (two different agent-host process trees) resolve
// to two different ids. findAgentHostAncestorId is the pure walk — this
// suite drives it with a fake process tree so it never spawns real `ps`.
describe('findAgentHostAncestorId', () => {
  function fakeReader(tree: Record<number, { ppid: number; command: string }>): ParentPidReader {
    return (pid) => tree[pid] ?? null
  }

  it('finds an agent-host ancestor a few hops up the parent chain', () => {
    const tree = {
      100: { ppid: 50, command: 'bash' },
      50: { ppid: 10, command: 'node' },
      10: { ppid: 1, command: 'claude' },
    }
    expect(findAgentHostAncestorId(100, fakeReader(tree))).toBe('ancestor-10')
  })

  it('matches the ancestor immediately when it is the starting process', () => {
    const tree = { 10: { ppid: 1, command: 'claude' } }
    expect(findAgentHostAncestorId(10, fakeReader(tree))).toBe('ancestor-10')
  })

  it('matches case-insensitively against a full app-bundle path', () => {
    const tree = {
      5: { ppid: 1, command: '/Applications/Claude Code.app/Contents/MacOS/Claude' },
    }
    expect(findAgentHostAncestorId(5, fakeReader(tree))).toBe('ancestor-5')
  })

  it('does not match a command that merely contains "claude" as a substring', () => {
    const tree = { 5: { ppid: 1, command: 'declaude-something' } }
    expect(findAgentHostAncestorId(5, fakeReader(tree))).toBeNull()
  })

  it('respects the depth bound — stops before reaching a match past maxDepth', () => {
    const tree = {
      100: { ppid: 50, command: 'bash' },
      50: { ppid: 10, command: 'node' },
      10: { ppid: 1, command: 'claude' },
    }
    // Only 2 iterations allowed: checks pid 100 then pid 50, never reaches
    // pid 10 where the match lives.
    expect(findAgentHostAncestorId(100, fakeReader(tree), 2)).toBeNull()
  })

  it('falls through cleanly when the reader returns null (process gone / unreadable)', () => {
    expect(findAgentHostAncestorId(999, fakeReader({}))).toBeNull()
  })

  it('falls through cleanly when the chain reaches pid 1 without a match', () => {
    const tree = {
      100: { ppid: 50, command: 'bash' },
      50: { ppid: 1, command: 'launchd' },
    }
    expect(findAgentHostAncestorId(100, fakeReader(tree))).toBeNull()
  })

  it('does not loop forever when a process reports itself as its own parent', () => {
    const tree = { 100: { ppid: 100, command: 'weird' } }
    expect(findAgentHostAncestorId(100, fakeReader(tree))).toBeNull()
  })
})
