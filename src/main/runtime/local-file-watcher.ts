import { createHash } from 'crypto'
import { readFileSync, watch, type FSWatcher } from 'fs'
import { basename, dirname } from 'path'

type OnFilesChanged = (entityIds: string[]) => void

interface WatchPair {
  dirWatcher: FSWatcher
  fileWatcher: FSWatcher | null
}

const fileReloadVersions = new Map<string, number>()
const fileToEntityIds = new Map<string, Set<string>>()
const fileWatchers = new Map<string, WatchPair>()
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
  for (const pair of fileWatchers.values()) {
    try { pair.dirWatcher.close() } catch {}
    try { pair.fileWatcher?.close() } catch {}
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

// Hybrid watch: a directory watch plus a file watch, because neither alone
// covers both save styles.
//
// - A watch on the file itself dies silently on an atomic save (editors and
//   agents write a temp file, then rename it over the target — the watched
//   inode no longer has that path).
// - A watch on the parent directory survives every rename inside it, but on
//   macOS fs.watch on a directory does not reliably report in-place content
//   writes to files within it (documented Node/FSEvents caveat) — so direct
//   overwrites (`cat new > file`, fs.writeFile) would go unseen.
//
// So the directory watch is the rename/create signal, and it re-arms a
// file-level watch onto the (possibly new) inode for the in-place-write
// signal. Both feed the same debounce + content-hash dedup, so double-fires
// collapse to one reload.
function startWatcher(localPath: string): void {
  const dir = dirname(localPath)
  const targetName = basename(localPath)
  try {
    const dirWatcher = watch(dir, (_eventType, filename) => {
      if (filename !== null && filename !== targetName) return
      rearmFileWatch(localPath)
      scheduleChangeHandling(localPath)
    })
    dirWatcher.on('error', () => stopWatcher(localPath))
    fileWatchers.set(localPath, { dirWatcher, fileWatcher: null })
    rearmFileWatch(localPath)
  } catch {
    // directory doesn't exist or cannot be watched — skip silently
  }
}

// (Re)attach the file-level watch to whatever inode currently sits at the
// path. Failure is fine (file may not exist yet — e.g. mid atomic save); the
// next directory event retries.
function rearmFileWatch(localPath: string): void {
  const pair = fileWatchers.get(localPath)
  if (!pair) return
  try { pair.fileWatcher?.close() } catch {}
  pair.fileWatcher = null
  try {
    const fileWatcher = watch(localPath, () => scheduleChangeHandling(localPath))
    // Swallow errors (e.g. the file vanishing) — the dir watcher owns
    // lifecycle and will re-arm on the next event for this filename.
    fileWatcher.on('error', () => {
      try { fileWatcher.close() } catch {}
      if (pair.fileWatcher === fileWatcher) pair.fileWatcher = null
    })
    pair.fileWatcher = fileWatcher
  } catch {
    // file doesn't exist right now — retry on the next dir event
  }
}

function stopWatcher(localPath: string): void {
  const pair = fileWatchers.get(localPath)
  if (pair) {
    try { pair.dirWatcher.close() } catch {}
    try { pair.fileWatcher?.close() } catch {}
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
