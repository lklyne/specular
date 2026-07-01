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
