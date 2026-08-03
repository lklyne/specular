/**
 * The single funnel every space-relative path resolves through (ADR 0033).
 *
 * A space is a folder the user picks: `.canvas` files, `assets/`, and
 * `.specular/` metadata sit directly inside it. When no folder has been
 * chosen, the space resolves to the pre-ADR-0033 location so existing
 * installs keep working untouched.
 */

import { app } from 'electron'
import { accessSync, constants as fsConstants, statSync } from 'fs'
import { join } from 'path'
import { getSpacePath } from './preferences'
import { DEFAULT_WORKSPACE_ID } from './space-persistence'

const LEGACY_WORKSPACES_DIR = 'workspaces'

/**
 * Resolves the current space's root folder. A resolver rather than cached
 * state: multi-window support (ADR 0033 §2) will need this to accept a
 * window argument, and a live re-point after onboarding must see a freshly
 * set `spacePath` without a process restart.
 */
export function spaceDir(): string {
  return getSpacePath() ?? join(app.getPath('userData'), LEGACY_WORKSPACES_DIR, DEFAULT_WORKSPACE_ID)
}

/**
 * Whether `path` is a readable/writable directory — the check that gates
 * boot (ADR 0033 §4). A configured space that fails this must prompt, never
 * silently fall back: an unmounted drive or a renamed folder is a question,
 * not a recoverable condition.
 */
export function isSpaceAvailable(path: string): boolean {
  try {
    if (!statSync(path).isDirectory()) return false
    accessSync(path, fsConstants.R_OK | fsConstants.W_OK)
    return true
  } catch {
    return false
  }
}
