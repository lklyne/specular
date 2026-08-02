/**
 * The single funnel every space-relative path resolves through (ADR 0033).
 *
 * A space is a folder the user picks: `.canvas` files, `assets/`, and
 * `.specular/` metadata sit directly inside it. When no folder has been
 * chosen, the space resolves to the pre-ADR-0033 location so existing
 * installs keep working untouched.
 */

import { app } from 'electron'
import { join } from 'path'
import { getSpacePath } from './preferences'
import { DEFAULT_WORKSPACE_ID } from './workspace-persistence'

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
