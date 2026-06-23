interface PasteData {
  readonly types: readonly string[]
  getData(type: string): string
}

export interface DetectedContent {
  lang: string
  text: string
}

function tryFormatJson(raw: string): string | null {
  const trimmed = raw.trim()
  const isStructured =
    (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
    (trimmed.startsWith('[') && trimmed.endsWith(']'))
  if (!isStructured) return null
  try {
    return JSON.stringify(JSON.parse(trimmed), null, 2)
  } catch {
    return null
  }
}

export function detectStructuredContent(data: PasteData): DetectedContent | null {
  const types = Array.from(data.types)

  if (types.includes('image/svg+xml')) {
    const text = (data.getData('image/svg+xml') || data.getData('text/plain')).trim()
    if (text) return { lang: 'svg', text }
  }

  if (types.includes('application/json')) {
    const raw = data.getData('application/json') || data.getData('text/plain')
    const json = raw ? tryFormatJson(raw) : null
    if (json) return { lang: 'json', text: json }
  }

  const plain = data.getData('text/plain')
  if (!plain) return null
  const trimmed = plain.trim()
  if (!trimmed) return null

  if (/^(?:<\?xml[^>]*\?>\s*)?<svg[\s>]/i.test(trimmed) && /<\/svg\s*>$/i.test(trimmed)) {
    return { lang: 'svg', text: trimmed }
  }

  const json = tryFormatJson(trimmed)
  if (json) return { lang: 'json', text: json }

  if (/^<!doctype\s+html/i.test(trimmed) || /^<html[\s>]/i.test(trimmed)) {
    return { lang: 'html', text: trimmed }
  }

  return null
}

export function wrapInFence(lang: string, text: string): string {
  let longestRun = 0
  for (const match of text.matchAll(/`+/g)) {
    longestRun = Math.max(longestRun, match[0].length)
  }
  const fence = '`'.repeat(Math.max(3, longestRun + 1))
  return `${fence}${lang}\n${text}\n${fence}`
}
