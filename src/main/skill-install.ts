import { app } from 'electron'
import { homedir } from 'os'
import { join } from 'path'
import { createHash } from 'crypto'
import {
  cpSync,
  existsSync,
  readFileSync,
  rmSync,
} from 'fs'

export type SkillId = 'specular'

const SKILL_FILENAME = 'SKILL.md'

/** Name of the (no-longer-shipped) agent-browser skill directory. Kept as a
 *  plain string rather than a `SkillId` member — it isn't installable
 *  through the generic skill flow anymore, but skill-migrations.ts still
 *  needs to read its bundled/installed hashes to run the one-time removal. */
const AGENT_BROWSER_SKILL_DIR_NAME = 'agent-browser'

function bundledSkillDirNamed(dirName: string): string {
  return app.isPackaged
    ? join(process.resourcesPath, 'skills', dirName)
    : join(process.cwd(), 'resources', 'skills', dirName)
}

function bundledSkillPathNamed(dirName: string): string {
  return join(bundledSkillDirNamed(dirName), SKILL_FILENAME)
}

export function claudeSkillsDir(): string {
  return join(homedir(), '.claude', 'skills')
}

function installedSkillDirNamed(dirName: string): string {
  return join(claudeSkillsDir(), dirName)
}

function installedSkillPathNamed(dirName: string): string {
  return join(installedSkillDirNamed(dirName), SKILL_FILENAME)
}

export function claudeDirExists(): boolean {
  return existsSync(join(homedir(), '.claude'))
}

export function sha256(data: Buffer): string {
  return createHash('sha256').update(data).digest('hex')
}

export function bundledSkillHash(skillId: SkillId): string | null {
  return hashSkillFile(skillId, 'bundled')
}

export function installedSkillHash(skillId: SkillId): string | null {
  return hashSkillFile(skillId, 'installed')
}

function readFileOrNull(path: string): Buffer | null {
  try {
    return readFileSync(path)
  } catch {
    return null
  }
}

/** Shared body behind the bundled/installed hash getters, for both the
 *  generic skill flow and the agent-browser migration helpers below. */
function hashSkillFile(dirName: string, which: 'bundled' | 'installed'): string | null {
  const path = which === 'bundled' ? bundledSkillPathNamed(dirName) : installedSkillPathNamed(dirName)
  const data = readFileOrNull(path)
  return data ? sha256(data) : null
}

export type SkillStatus =
  | { kind: 'installed' }
  | { kind: 'outdated'; detail: string }
  | { kind: 'missing' }
  | { kind: 'blocked'; detail: string }

export function getSkillStatus(skillId: SkillId): SkillStatus {
  const bundled = readFileOrNull(bundledSkillPathNamed(skillId))
  if (!bundled) {
    return {
      kind: 'blocked',
      detail: `Bundled skill source missing at ${bundledSkillPathNamed(skillId)}`,
    }
  }
  const installed = readFileOrNull(installedSkillPathNamed(skillId))
  if (!installed) return { kind: 'missing' }
  if (sha256(bundled) === sha256(installed)) return { kind: 'installed' }
  return { kind: 'outdated', detail: 'Installed skill differs from bundled version.' }
}

export interface SkillInstallResult {
  success: boolean
  message: string
}

export function installSkill(skillId: SkillId): SkillInstallResult {
  const srcDir = bundledSkillDirNamed(skillId)
  const srcFile = bundledSkillPathNamed(skillId)
  if (!existsSync(srcFile)) {
    return { success: false, message: `Bundled skill source missing at ${srcFile}` }
  }
  try {
    cpSync(srcDir, installedSkillDirNamed(skillId), { recursive: true })
    return {
      success: true,
      message: `${skillId} skill installed at ${installedSkillPathNamed(skillId)}.`,
    }
  } catch (error) {
    return {
      success: false,
      message: `Failed to install ${skillId} skill: ${(error as Error).message}`,
    }
  }
}

export function uninstallSkill(skillId: SkillId): SkillInstallResult {
  const dir = installedSkillDirNamed(skillId)
  if (!existsSync(dir)) {
    return { success: true, message: `${skillId} skill was not installed.` }
  }
  try {
    rmSync(dir, { recursive: true, force: true })
    return { success: true, message: `${skillId} skill removed from ${dir}.` }
  } catch (error) {
    return {
      success: false,
      message: `Failed to remove ${skillId} skill: ${(error as Error).message}`,
    }
  }
}

// --- agent-browser skill migration helpers (D2) ---
//
// The agent-browser skill stub is no longer installed through the generic
// flow above, but the one-time removal migration (skill-migrations.ts)
// still needs to read its bundled/installed hashes and know where it lives
// on disk to decide whether it's safe to delete.

export function bundledAgentBrowserSkillHash(): string | null {
  return hashSkillFile(AGENT_BROWSER_SKILL_DIR_NAME, 'bundled')
}

export function installedAgentBrowserSkillHash(): string | null {
  return hashSkillFile(AGENT_BROWSER_SKILL_DIR_NAME, 'installed')
}

export function installedAgentBrowserSkillDir(): string {
  return installedSkillDirNamed(AGENT_BROWSER_SKILL_DIR_NAME)
}
