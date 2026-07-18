import { createHash } from 'crypto'
import { readFileSync, watch, type FSWatcher } from 'fs'
import { basename, dirname } from 'path'

type OnFilesChanged = (entityIds: string[]) => void

const fileReloadVersions = new Map<string, number>()
const fileToEntityIds = new Map<string, Set<string>>()
const fileWatchers = new Map<string, FSWatcher>()
const fileHashes = new Map<string, string>()
const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>()

let onChangedCallback: OnFilesChanged | null = null

export function initFileWatcher(onChanged: OnFilesChanged): void {
  onChangedCallback = onChanged
}

export function getFileReloadVersion(entityId: string): number {
  return fileReloadVersions.get(entityId) ?? 0
}

/** Force a reload signal for one entity without waiting on the watcher —
 *  the manual "Refresh" action. */
export function bumpFileReloadVersion(entityId: string): void {
  fileReloadVersions.set(entityId, (fileReloadVersions.get(entityId) ?? 0) + 1)
  onChangedCallback?.([entityId])
}

export function watchEntityFile(entityId: string, filePath: string): void {
  const localPath = toLocalPath(filePath)
  if (!localPath) return

  let ids = fileToEntityIds.get(localPath)
  if (!ids) {
    ids = new Set()
    fileToEntityIds.set(localPath, ids)
  }
  ids.add(entityId)

  if (!fileWatchers.has(localPath)) {
    startWatcher(localPath)
  }
}

export function unwatchEntityFile(entityId: string, filePath: string): void {
  const localPath = toLocalPath(filePath)
  if (!localPath) return

  const ids = fileToEntityIds.get(localPath)
  if (!ids) return
  ids.delete(entityId)
  fileReloadVersions.delete(entityId)

  if (ids.size === 0) {
    fileToEntityIds.delete(localPath)
    stopWatcher(localPath)
  }
}

export function teardownAllFileWatchers(): void {
  for (const watcher of fileWatchers.values()) {
    try { watcher.close() } catch {}
  }
  fileWatchers.clear()
  fileToEntityIds.clear()
  fileReloadVersions.clear()
  fileHashes.clear()
  for (const timer of debounceTimers.values()) clearTimeout(timer)
  debounceTimers.clear()
}

function toLocalPath(filePath: string): string | null {
  if (filePath.startsWith('http://') || filePath.startsWith('https://')) return null
  if (filePath.startsWith('local-file://')) return decodeURIComponent(filePath.slice('local-file://'.length))
  return filePath
}

// Watches the parent directory rather than the file itself. Editors and
// agents commonly save atomically (write a temp file, then rename it over
// the target), which replaces the target's inode — a watch on the file
// itself fires once for the rename and then silently stops reporting
// changes, since the inode it's attached to no longer has that path. The
// directory's inode survives every rename inside it, so watching the
// directory and filtering by filename keeps reporting changes across
// atomic saves without needing to re-arm anything.
function startWatcher(localPath: string): void {
  const dir = dirname(localPath)
  const targetName = basename(localPath)
  try {
    const watcher = watch(dir, (_eventType, filename) => {
      if (filename !== null && filename !== targetName) return
      scheduleChangeHandling(localPath)
    })
    watcher.on('error', () => stopWatcher(localPath))
    fileWatchers.set(localPath, watcher)
  } catch {
    // directory doesn't exist or cannot be watched — skip silently
  }
}

function stopWatcher(localPath: string): void {
  const watcher = fileWatchers.get(localPath)
  if (watcher) {
    try { watcher.close() } catch {}
    fileWatchers.delete(localPath)
  }
  const timer = debounceTimers.get(localPath)
  if (timer) {
    clearTimeout(timer)
    debounceTimers.delete(localPath)
  }
  fileHashes.delete(localPath)
}

function scheduleChangeHandling(localPath: string): void {
  const existing = debounceTimers.get(localPath)
  if (existing) clearTimeout(existing)
  const timer = setTimeout(() => {
    debounceTimers.delete(localPath)
    handleFileChange(localPath)
  }, 80)
  debounceTimers.set(localPath, timer)
}

function handleFileChange(localPath: string): void {
  let content: Buffer
  try {
    content = readFileSync(localPath)
  } catch {
    return
  }
  const hash = createHash('md5').update(content).digest('hex')
  if (fileHashes.get(localPath) === hash) return
  fileHashes.set(localPath, hash)

  const ids = fileToEntityIds.get(localPath)
  if (!ids || ids.size === 0) return

  const changedEntityIds: string[] = []
  for (const entityId of ids) {
    fileReloadVersions.set(entityId, (fileReloadVersions.get(entityId) ?? 0) + 1)
    changedEntityIds.push(entityId)
  }
  onChangedCallback?.(changedEntityIds)
}
