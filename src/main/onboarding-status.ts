import type {
  OnboardingComponentStatus,
  OnboardingStatusSnapshot,
} from '../shared/types'
import { isCliInstalled } from './cli-install'
import { claudeDirExists, getSkillStatus, type SkillStatus } from './skill-install'
import { getAgentBrowserStatus } from './agent-browser-install'

function cliStatus(): OnboardingComponentStatus {
  const result = isCliInstalled()
  if (!result.installed) return { kind: 'missing' }
  if (result.needsPathUpdate) {
    return {
      kind: 'installed',
      detail: `Installed at ${result.path} — add ~/.local/bin to PATH to invoke.`,
    }
  }
  return { kind: 'installed', detail: `Installed at ${result.path}` }
}

function skillToStatus(status: SkillStatus): OnboardingComponentStatus {
  switch (status.kind) {
    case 'installed':
      return { kind: 'installed' }
    case 'missing':
      return { kind: 'missing' }
    case 'outdated':
      return { kind: 'outdated', detail: status.detail }
    case 'blocked':
      return { kind: 'blocked', detail: status.detail }
  }
}

/**
 * agent-browser is bundled and auto-configured on launch (configureBundledAgentBrowser
 * in index.ts) — there's nothing for the user to install or toggle. This status is
 * display-only: bundled driver readiness, plus a note when a separate user-owned
 * binary is also on PATH (see D3, issue #318).
 */
function agentBrowserStatus(agent: Awaited<ReturnType<typeof getAgentBrowserStatus>>): OnboardingComponentStatus {
  if (agent.binary.kind === 'installed') {
    const detail = agent.userInstall
      ? `Bundled ${agent.binary.version}; user install ${agent.userInstall.version} also detected on PATH.`
      : `Bundled ${agent.binary.version}.`
    return { kind: 'installed', detail }
  }
  if (agent.binary.kind === 'blocked') {
    return { kind: 'blocked', detail: agent.binary.detail }
  }
  return { kind: 'missing' }
}

/**
 * Probing the agent-browser binary means spawning it, which can stall out to
 * its version timeout. Every other field is a cheap sync fs check, so the probe
 * result is held here and refreshed deliberately — windows read a snapshot, they
 * never wait on a subprocess to open.
 */
let agentBrowser: OnboardingComponentStatus = { kind: 'missing' }
const listeners = new Set<(status: OnboardingStatusSnapshot) => void>()

export function getOnboardingStatus(): OnboardingStatusSnapshot {
  return {
    cli: cliStatus(),
    skill: skillToStatus(getSkillStatus('specular')),
    agentBrowser,
    claudeDirExists: claudeDirExists(),
  }
}

export async function refreshOnboardingStatus(): Promise<OnboardingStatusSnapshot> {
  agentBrowser = agentBrowserStatus(await getAgentBrowserStatus())
  const status = getOnboardingStatus()
  for (const listener of listeners) listener(status)
  return status
}

export function onOnboardingStatusChanged(
  listener: (status: OnboardingStatusSnapshot) => void,
): void {
  listeners.add(listener)
}
