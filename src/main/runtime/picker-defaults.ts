/**
 * Starting directories for the folder pickers.
 *
 * Electron 43 defaults `defaultPath` to the Downloads folder when a dialog
 * doesn't set one, which is the wrong neighbourhood for both things we ask
 * users to point at: a space folder lives near their other spaces, and a
 * repo lives near their other repos. Each picker opens beside the closest
 * thing we already know about, falling back to home.
 */

import { app } from 'electron'
import { existsSync, statSync } from 'fs'
import { dirname } from 'path'
import { listRepos } from './dev-server-manager'
import { getSpacePath } from './preferences'

function firstExistingDir(...candidates: (string | undefined)[]): string {
  for (const candidate of candidates) {
    if (!candidate) continue
    try {
      if (existsSync(candidate) && statSync(candidate).isDirectory()) return candidate
    } catch {
      // Unreadable or on a disconnected drive — try the next candidate.
    }
  }
  return app.getPath('home')
}

/**
 * Where "choose a space folder" opens: alongside the current space, so the
 * sibling spaces a user is switching between are one click away. Falls back
 * to home on first run, when there's no configured space yet.
 */
export function spacePickerDefaultPath(): string {
  const configured = getSpacePath()
  return firstExistingDir(configured && dirname(configured))
}

/**
 * Where "connect a repo" opens: alongside the most recently connected repo,
 * since projects cluster in one directory.
 */
export function repoPickerDefaultPath(): string {
  const repos = listRepos()
  const mostRecent = repos[repos.length - 1]
  return firstExistingDir(mostRecent && dirname(mostRecent.absolutePath))
}
