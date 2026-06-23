const LONG_TEXT_THRESHOLD = 300

export function shouldRouteTextToDocument(text: string): boolean {
  if (text.length > LONG_TEXT_THRESHOLD) return true
  return /^#{1,6}\s/m.test(text)
    || /^\|.+\|/m.test(text)
    || /```/.test(text)
}

export function deriveDocumentName(text: string): string {
  const heading = text.match(/^#{1,6}\s+(.+)/m)
  if (heading) return heading[1].slice(0, 60)
  return text.split('\n')[0].trim().slice(0, 60) || 'Note'
}
