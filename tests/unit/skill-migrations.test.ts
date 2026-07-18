/**
 * agent-browser skill removal migration (D2, issue #318).
 *
 * Drives `runAgentBrowserSkillRemovalMigration` from
 * src/main/skill-migrations.ts directly through its injected deps (state
 * load/save, hash readers, user-binary detector, dir remover) — no fs,
 * electron, or child_process boundary crossed, per tests/README.md's
 * "mock at process boundaries, not internal collaborators" rule.
 *
 * Mutation-verified by deleting the `if (installed === null)` early-return
 * branch in production code — "leaves the skill alone when nothing is
 * installed" then fails because `removeDir` gets called on a hash of `null`
 * matching nothing, which the guard is specifically there to prevent.
 */

import { describe, expect, it, vi } from 'vitest'
import type { OnboardingState } from '../../src/shared/types'
import {
  runAgentBrowserSkillRemovalMigration,
  type AgentBrowserSkillMigrationDeps,
} from '../../src/main/skill-migrations'

function makeDeps(overrides: Partial<AgentBrowserSkillMigrationDeps> = {}): {
  deps: AgentBrowserSkillMigrationDeps
  savedStates: OnboardingState[]
  removedDirs: string[]
} {
  const savedStates: OnboardingState[] = []
  const removedDirs: string[] = []
  let state: OnboardingState = { completed: true }

  const deps: AgentBrowserSkillMigrationDeps = {
    loadState: () => state,
    saveState: (next) => {
      state = next
      savedStates.push(next)
    },
    installedHash: () => 'installed-hash',
    bundledHash: () => 'bundled-hash',
    installedDir: () => '/home/user/.claude/skills/agent-browser',
    hasUserBinary: async () => false,
    removeDir: (dir) => removedDirs.push(dir),
    ...overrides,
  }
  return { deps, savedStates, removedDirs }
}

describe('runAgentBrowserSkillRemovalMigration', () => {
  it('removes the skill when the recorded install hash matches, no user binary is on PATH, and the migration has not run yet', async () => {
    const { deps, savedStates, removedDirs } = makeDeps({
      loadState: () => ({
        completed: true,
        skillHashes: { 'agent-browser': 'installed-hash' },
      }),
      installedHash: () => 'installed-hash',
      bundledHash: () => 'a-different-bundled-hash',
      hasUserBinary: async () => false,
    })

    const outcome = await runAgentBrowserSkillRemovalMigration(deps)

    expect(outcome).toBe('removed')
    expect(removedDirs).toEqual(['/home/user/.claude/skills/agent-browser'])
    expect(savedStates).toHaveLength(1)
    expect(savedStates[0].agentBrowserSkillMigrationDone).toBe(true)
  })

  it('removes the skill when the installed hash matches only the current bundled stub (no recorded hash)', async () => {
    const { deps, removedDirs } = makeDeps({
      loadState: () => ({ completed: true }),
      installedHash: () => 'bundled-hash',
      bundledHash: () => 'bundled-hash',
      hasUserBinary: async () => false,
    })

    const outcome = await runAgentBrowserSkillRemovalMigration(deps)

    expect(outcome).toBe('removed')
    expect(removedDirs).toHaveLength(1)
  })

  it('leaves the skill alone when nothing is installed', async () => {
    const { deps, removedDirs, savedStates } = makeDeps({
      loadState: () => ({ completed: true }),
      installedHash: () => null,
    })

    const outcome = await runAgentBrowserSkillRemovalMigration(deps)

    expect(outcome).toBe('left-not-installed')
    expect(removedDirs).toEqual([])
    expect(savedStates[0].agentBrowserSkillMigrationDone).toBe(true)
  })

  it('leaves the skill alone when the installed content matches neither the recorded nor the bundled hash (hand-edited or foreign)', async () => {
    const { deps, removedDirs, savedStates } = makeDeps({
      loadState: () => ({
        completed: true,
        skillHashes: { 'agent-browser': 'some-other-hash' },
      }),
      installedHash: () => 'hand-edited-hash',
      bundledHash: () => 'bundled-hash',
    })

    const outcome = await runAgentBrowserSkillRemovalMigration(deps)

    expect(outcome).toBe('left-hash-mismatch')
    expect(removedDirs).toEqual([])
    expect(savedStates[0].agentBrowserSkillMigrationDone).toBe(true)
  })

  it('leaves the skill alone when a user-owned agent-browser binary is on PATH, even if the hash matches', async () => {
    const { deps, removedDirs, savedStates } = makeDeps({
      loadState: () => ({
        completed: true,
        skillHashes: { 'agent-browser': 'installed-hash' },
      }),
      installedHash: () => 'installed-hash',
      hasUserBinary: async () => true,
    })

    const outcome = await runAgentBrowserSkillRemovalMigration(deps)

    expect(outcome).toBe('left-user-binary-present')
    expect(removedDirs).toEqual([])
    expect(savedStates[0].agentBrowserSkillMigrationDone).toBe(true)
  })

  it('does nothing and does not re-save state once the migration has already run', async () => {
    const { deps, removedDirs, savedStates } = makeDeps({
      loadState: () => ({ completed: true, agentBrowserSkillMigrationDone: true }),
      installedHash: () => 'installed-hash',
      hasUserBinary: vi.fn(async () => false),
    })

    const outcome = await runAgentBrowserSkillRemovalMigration(deps)

    expect(outcome).toBe('already-done')
    expect(removedDirs).toEqual([])
    expect(savedStates).toEqual([])
    expect(deps.hasUserBinary).not.toHaveBeenCalled()
  })

  it('records the migration as done even when the removal itself throws', async () => {
    const { deps, savedStates } = makeDeps({
      loadState: () => ({
        completed: true,
        skillHashes: { 'agent-browser': 'installed-hash' },
      }),
      installedHash: () => 'installed-hash',
      hasUserBinary: async () => false,
      removeDir: () => {
        throw new Error('EACCES: permission denied')
      },
    })

    const outcome = await runAgentBrowserSkillRemovalMigration(deps)

    expect(outcome).toBe('remove-failed')
    expect(savedStates[0].agentBrowserSkillMigrationDone).toBe(true)
  })
})
