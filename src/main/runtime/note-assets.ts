import { existsSync, mkdirSync, writeFileSync, readFileSync, renameSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'
import { DEFAULT_WORKSPACE_ID } from './workspace-persistence'

function workspaceNoteDir(): string {
  const dir = join(app.getPath('userData'), 'workspaces', DEFAULT_WORKSPACE_ID)
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

function sanitizeNoteName(name: string): string {
  return name.replace(/[/\\:*?"<>|]/g, '_').trim() || 'Untitled Note'
}

/**
 * Create a new .md note file in the workspace directory.
 * Returns the absolute file path.
 */
export function createNoteFile(name?: string, initialContent?: string): string {
  const dir = workspaceNoteDir()
  const baseName = sanitizeNoteName(name ?? 'Untitled Note')
  let fileName = `${baseName}.md`
  let filePath = join(dir, fileName)

  // Handle name collisions with numeric suffix
  let counter = 2
  while (existsSync(filePath)) {
    fileName = `${baseName} ${counter}.md`
    filePath = join(dir, fileName)
    counter++
  }

  writeFileSync(filePath, initialContent ?? '', 'utf8')
  return filePath
}

/**
 * Read a note file's content.
 */
export function readNoteFile(filePath: string): string | null {
  if (!existsSync(filePath)) return null
  try {
    return readFileSync(filePath, 'utf8')
  } catch {
    return null
  }
}

/**
 * Write content to a note file atomically.
 */
export function writeNoteFile(filePath: string, content: string): void {
  const tmpFile = `${filePath}.tmp`
  writeFileSync(tmpFile, content, 'utf8')
  renameSync(tmpFile, filePath)
}

/**
 * Rename a note file on disk, preserving its existing extension.
 * Returns the new absolute file path, or null if the rename failed.
 */
export function renameNoteFile(oldFilePath: string, newName: string): string | null {
  const dir = join(oldFilePath, '..')
  const baseName = oldFilePath.split('/').pop() ?? oldFilePath
  const ext = /\.md$/i.test(baseName) ? '.md' : ''
  const sanitized = sanitizeNoteName(newName)
  let fileName = `${sanitized}${ext}`
  let newPath = join(dir, fileName)

  let counter = 2
  while (existsSync(newPath) && newPath !== oldFilePath) {
    fileName = `${sanitized} ${counter}${ext}`
    newPath = join(dir, fileName)
    counter++
  }

  if (newPath === oldFilePath) return oldFilePath

  try {
    renameSync(oldFilePath, newPath)
    return newPath
  } catch {
    return null
  }
}

/**
 * Whether a file path is a renamable workspace-managed note (markdown).
 */
export function isRenamableNotePath(filePath: string): boolean {
  return /\.md$/i.test(filePath)
}
