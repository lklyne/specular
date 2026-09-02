/**
 * Silent skill updates on app launch.
 *
 * Rule: only auto-overwrite the user's installed skill file when its content
 * matches the hash we recorded the last time we wrote it ourselves. Any other
 * mismatch means the user (or another tool) edited the file — leave it alone
 * and let the Setup window surface the drift.
 */

import { loadOnboardingState, saveOnboardingState } from './runtime/preferences'
import {
  bundledSkillHash,
  installSkill,
  installedSkillHash,
  type SkillId,
} from './skill-install'

const SKILL_IDS: SkillId[] = ['specular']

export interface AutoUpdateOutcome {
  updated: SkillId[]
  skippedDueToDrift: SkillId[]
}

export function autoUpdateSkillsIfSafe(): AutoUpdateOutcome {
  const state = loadOnboardingState()
  const recorded = state.skillHashes ?? {}
  const updated: SkillId[] = []
  const skippedDueToDrift: SkillId[] = []
  const nextRecorded = { ...recorded }

  for (const skillId of SKILL_IDS) {
    const bundled = bundledSkillHash(skillId)
    const installed = installedSkillHash(skillId)
    if (!bundled) continue
    if (installed === null) continue // not installed at all — onboarding handles it
    if (installed === bundled) {
      if (recorded[skillId] !== bundled) nextRecorded[skillId] = bundled
      continue
    }
    if (recorded[skillId] && recorded[skillId] === installed) {
      const result = installSkill(skillId)
      if (result.success) {
        nextRecorded[skillId] = bundled
        updated.push(skillId)
      } else {
        skippedDueToDrift.push(skillId)
      }
    } else {
      skippedDueToDrift.push(skillId)
    }
  }

  if (
    JSON.stringify(nextRecorded) !== JSON.stringify(recorded) ||
    updated.length > 0
  ) {
    saveOnboardingState({ ...state, skillHashes: nextRecorded })
  }

  return { updated, skippedDueToDrift }
}

/** Record a skill's bundled hash after a fresh install completes. */
export function recordInstalledSkillHash(skillId: SkillId): void {
  const bundled = bundledSkillHash(skillId)
  if (!bundled) return
  const state = loadOnboardingState()
  saveOnboardingState({
    ...state,
    skillHashes: { ...(state.skillHashes ?? {}), [skillId]: bundled },
  })
}
