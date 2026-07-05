export function filePathToSrc(filePath: string): string {
  if (
    filePath.startsWith('local-file://') ||
    filePath.startsWith('http://') ||
    filePath.startsWith('https://')
  ) {
    return filePath
  }
  return `local-file://${filePath}`
}

/**
 * Cache-busted src for live-reloading file entities. Remounting an <iframe>/<img>
 * with an identical local-file:// URL serves the cached response, so we append
 * the reload version to force a fresh fetch when the file changes on disk.
 */
export function filePathToSrcVersioned(filePath: string, version = 0): string {
  const src = filePathToSrc(filePath)
  if (!version) return src
  return `${src}${src.includes('?') ? '&' : '?'}v=${version}`
}

export interface RendererFileApi {
  showFileInFinder: (path: string) => void
  copyFileAsPng: (path: string) => void
  reorderStack: (
    action: 'bring-forward' | 'send-backward' | 'bring-to-front' | 'send-to-back',
    targetId?: string,
  ) => void
  writeNoteFile: (path: string, content: string) => Promise<boolean>
  applyNoteContent: (entityId: string, content: string) => Promise<boolean>
}

export function getFileApi(): RendererFileApi {
  return (window as unknown as { electronAPI: RendererFileApi }).electronAPI
}
