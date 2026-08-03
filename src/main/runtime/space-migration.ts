/**
 * Copy-verify-delete migration between two space folders (ADR 0033 §3).
 *
 * Pure Node — no Electron imports — so it's testable without the app runtime.
 * Every file is copied and verified (existence + byte size) before any
 * original is touched; a verification failure throws and leaves every
 * original in place. A rename would be cheaper but can't cross volumes, and
 * silently losing a user's files to a partial move is worse than the extra
 * I/O of a copy.
 */

import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync, statSync, unlinkSync } from 'fs'
import { dirname, join } from 'path'

const SPACE_META_DIR = '.specular'
const ASSETS_DIR = 'assets'

function listFilesRecursive(dir: string): string[] {
  if (!existsSync(dir)) return []
  const out: string[] = []
  const walk = (relDir: string): void => {
    const absDir = join(dir, relDir)
    for (const entry of readdirSync(absDir, { withFileTypes: true })) {
      const rel = relDir ? join(relDir, entry.name) : entry.name
      if (entry.isDirectory()) {
        walk(rel)
      } else if (entry.isFile()) {
        out.push(rel)
      }
    }
  }
  walk('')
  return out
}

/**
 * Every path (relative to `spacePath`) that constitutes space content:
 * root-level `.canvas` files, root-level `.md` notes (written to the space
 * root — see note-assets.ts — even though the ADR text only calls out the
 * main three), `assets/`, and `.specular/`.
 */
function spaceRelativeFiles(spacePath: string): string[] {
  const files: string[] = []
  if (existsSync(spacePath)) {
    for (const entry of readdirSync(spacePath, { withFileTypes: true })) {
      if (entry.isFile() && (entry.name.endsWith('.canvas') || entry.name.endsWith('.md'))) {
        files.push(entry.name)
      }
    }
  }
  for (const subdir of [ASSETS_DIR, SPACE_META_DIR]) {
    for (const rel of listFilesRecursive(join(spacePath, subdir))) {
      files.push(join(subdir, rel))
    }
  }
  return files
}

/** Whether `spacePath` has any `.canvas` files directly inside it — the
 *  signal used to distinguish "someone re-opening an existing space" from
 *  "an empty destination" (ADR 0033 §3). */
export function hasCanvasFiles(spacePath: string): boolean {
  if (!existsSync(spacePath)) return false
  return readdirSync(spacePath, { withFileTypes: true }).some(
    (entry) => entry.isFile() && entry.name.endsWith('.canvas'),
  )
}

/**
 * Copy every `.canvas`/`.md` file, `assets/`, and `.specular/` from `fromDir`
 * to `toDir`, verify each copy, then delete the originals. Throws on any
 * verification failure and leaves `fromDir` untouched — the caller (§3's
 * "Move my canvases" flow) must not report success on a partial migration.
 */
export function migrateSpace(fromDir: string, toDir: string): void {
  const relFiles = spaceRelativeFiles(fromDir)
  if (!relFiles.length) return

  mkdirSync(toDir, { recursive: true })

  try {
    for (const rel of relFiles) {
      const src = join(fromDir, rel)
      const dest = join(toDir, rel)
      mkdirSync(dirname(dest), { recursive: true })
      copyFileSync(src, dest)
      if (!existsSync(dest)) {
        throw new Error(`copy did not produce a file at ${dest}`)
      }
      const srcSize = statSync(src).size
      const destSize = statSync(dest).size
      if (destSize !== srcSize) {
        throw new Error(
          `size mismatch for ${rel}: source ${srcSize} bytes, destination ${destSize} bytes`,
        )
      }
    }
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error)
    throw new Error(
      `Failed to migrate space from "${fromDir}" to "${toDir}": ${reason}. Originals left in place.`,
    )
  }

  // Every copy verified — safe to remove the originals.
  for (const rel of relFiles) {
    try {
      unlinkSync(join(fromDir, rel))
    } catch {
      // Best-effort: the copy already succeeded and is verified, so a
      // stray leftover original is clutter, not data loss.
    }
  }
  for (const subdir of [ASSETS_DIR, SPACE_META_DIR]) {
    try {
      rmSync(join(fromDir, subdir), { recursive: true, force: false })
    } catch {
      // Directory not empty (unexpected file) or already gone — leave it.
    }
  }
}
