import { isAbsolute } from 'path'
import { fileURLToPath } from 'url'

function pathFromToken(token: string): string | null {
  const trimmed = token.trim().replace(/^"|"$/g, '')
  if (!trimmed) return null
  if (/^file:\/\//i.test(trimmed)) {
    try {
      return fileURLToPath(trimmed)
    } catch {
      return null
    }
  }
  return isAbsolute(trimmed) ? trimmed : null
}

export function filePathsFromClipboardReferences(references: string[]): string[] {
  const seen = new Set<string>()
  const paths: string[] = []

  for (const reference of references) {
    for (const line of reference.split(/\r?\n|\0/)) {
      const trimmed = line.trim()
      if (!trimmed || trimmed.startsWith('#')) continue
      if (trimmed === 'copy' || trimmed === 'cut') continue

      const path = pathFromToken(trimmed)
      if (!path || seen.has(path)) continue
      seen.add(path)
      paths.push(path)
    }
  }

  return paths
}
