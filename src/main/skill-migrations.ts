/**
 * One-time migrations for skills we used to bundle/install and no longer do.
 *
 * The agent-browser skill stub (resources/skills/agent-browser/) told agents
 * to run `agent-browser skills get core` — which fails without a global
 * agent-browser install, and with one steers agents into driving a separate
 * Chromium disconnected from the canvas. Guidance now lives in the specular
 * skill instead. We stopped installing the stub, but existing installs need a
 * one-time cleanup pass rather than silent, repeated re-evaluation on every
 * launch.
 *
 * Deliberately electron-free: this module holds only the pure guard logic
 * behind an injected-deps seam, so it can be unit-tested without booting
 * Electron. The real deps (preferences store, skill hashes, binary
 * detection, fs) are wired up at the call site in src/main/index.ts, which
 * already depends on Electron for everything else at launch.
 */

import { breadcrumb } from './sentry-context'
import type { OnboardingState } from '../shared/types'

export type AgentBrowserSkillMigrationOutcome =
  | 'removed'
  | 'remove-failed'
  | 'left-not-installed'
  | 'left-hash-mismatch'
  | 'left-user-binary-present'
  | 'already-done'

export interface AgentBrowserSkillMigrationDeps {
  loadState: () => OnboardingState
  saveState: (next: OnboardingState) => void
  /** sha256 of the currently-installed ~/.claude/skills/agent-browser/SKILL.md, or null if absent. */
  installedHash: () => string | null
  /** sha256 of the bundled resources/skills/agent-browser/SKILL.md stub, or null if missing. */
  bundledHash: () => string | null
  installedDir: () => string
  hasUserBinary: () => Promise<boolean>
  removeDir: (dir: string) => void
}

/**
 * Removes the installed agent-browser skill directory only when every guard
 * holds, and always records that the one-time evaluation ran (whatever the
 * outcome) so it never re-runs. The recorded install-time hash is checked
 * first because packaged builds refetch the upstream stub at package time —
 * a released bundle's hash can differ from the hash committed to this repo,
 * so the bundled hash alone isn't a reliable "did we write this" signal.
 */
export async function runAgentBrowserSkillRemovalMigration(
  deps: AgentBrowserSkillMigrationDeps,
): Promise<AgentBrowserSkillMigrationOutcome> {
  const state = deps.loadState()
  if (state.agentBrowserSkillMigrationDone) return 'already-done'

  const installed = deps.installedHash()
  const recorded = state.skillHashes?.['agent-browser'] ?? null
  const bundled = deps.bundledHash()
  const weWroteIt = installed !== null && (installed === recorded || installed === bundled)

  let outcome: AgentBrowserSkillMigrationOutcome
  if (installed === null) {
    outcome = 'left-not-installed'
  } else if (!weWroteIt) {
    outcome = 'left-hash-mismatch'
  } else if (await deps.hasUserBinary()) {
    outcome = 'left-user-binary-present'
  } else {
    try {
      deps.removeDir(deps.installedDir())
      outcome = 'removed'
    } catch {
      // Filesystem-layer failure (permissions, concurrent modification, …).
      // Still a completed evaluation — a retry next launch is unlikely to
      // succeed for the same reason, so the flag is set regardless.
      outcome = 'remove-failed'
    }
  }

  deps.saveState({ ...state, agentBrowserSkillMigrationDone: true })
  breadcrumb('onboarding', 'agent-browser-skill-migration', { outcome })
  return outcome
}
