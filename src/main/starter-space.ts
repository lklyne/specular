import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'

/**
 * Seeds a first-time space with the starter canvas.
 *
 * The canvas is data, not code: it ships as a real `.canvas` file and is
 * copied into the space before the normal loader runs, so it is adopted by
 * the same path that reads every other canvas. Nothing about it is special
 * once it lands — the user can edit or delete it like any of their own.
 *
 * File nodes store an absolute path (see `serializeFileToFileNode`), which
 * a bundled file can't know ahead of time, so the note's path is written as
 * a token and resolved against the destination on copy.
 */

const SPACE_TOKEN = '__SPECULAR_SPACE__'
const STARTER_CANVAS = 'Welcome.canvas'
const STARTER_NOTE = 'Welcome.md'

function starterDir(): string {
  // Packaged builds carry the folder as an extraResource; dev runs read it
  // straight out of the repo.
  return app.isPackaged
    ? join(process.resourcesPath, 'starter-space')
    : join(app.getAppPath(), 'resources', 'starter-space')
}

function hasAnyCanvas(spacePath: string): boolean {
  try {
    return readdirSync(spacePath).some((name) => name.endsWith('.canvas'))
  } catch {
    return false
  }
}

/**
 * Copies the starter canvas and its note into `spacePath`, but only when the
 * space holds no canvases at all — an existing space is never touched, so a
 * user who deletes the starter does not get it back on the next launch.
 *
 * Returns true when files were written.
 */
export function seedStarterSpace(spacePath: string): boolean {
  if (hasAnyCanvas(spacePath)) return false

  try {
    const source = starterDir()
    const canvasSource = join(source, STARTER_CANVAS)
    const noteSource = join(source, STARTER_NOTE)
    if (!existsSync(canvasSource) || !existsSync(noteSource)) return false

    mkdirSync(spacePath, { recursive: true })
    const notePath = join(spacePath, STARTER_NOTE)
    copyFileSync(noteSource, notePath)
    const canvas = readFileSync(canvasSource, 'utf8').replaceAll(
      `${SPACE_TOKEN}/${STARTER_NOTE}`,
      notePath,
    )
    writeFileSync(join(spacePath, STARTER_CANVAS), canvas, 'utf8')
    return true
  } catch (error) {
    // A space we can't seed is still a usable empty space — never block boot.
    console.error('Could not seed the starter space:', error)
    return false
  }
}
