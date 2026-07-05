import { createHash } from 'crypto'
import { readFileSync, watch, type FSWatcher } from 'fs'

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

function startWatcher(localPath: string): void {
  try {
    const watcher = watch(localPath, () => scheduleChangeHandling(localPath))
    watcher.on('error', () => stopWatcher(localPath))
    fileWatchers.set(localPath, watcher)
  } catch {
    // file doesn't exist or cannot be watched — skip silently
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
