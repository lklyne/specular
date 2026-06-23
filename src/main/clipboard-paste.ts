import { clipboard } from 'electron'
import { existsSync } from 'fs'
import { DESKTOP_PRESET_INDEX } from '../shared/constants'
import { looksLikeUrl, normalizeUserUrl } from '../shared/url'
import type {
  ClipboardEntitySelectionPayload,
  ClipboardPageSelectionPayload,
} from '../shared/types'
import { createTextEntity } from './runtime/text-entity-state'
import {
  getStickyDefaultColor,
  getStickyDefaultSize,
} from './runtime/tool-defaults'
import { createFileEntity } from './runtime/document-commands'
import { saveImageBuffer } from './runtime/image-assets'
import { htmlDefaultSize, imageSizeFromPath, videoSizeFromPath } from './runtime/image-sizing'
import { createNoteFile } from './runtime/note-assets'
import { createPageAtPosition } from './workspace-pages'
import {
  pasteEntitiesFromClipboard,
  pastePagesFromClipboard,
} from './workspace-clipboard'
import { filePathsFromClipboardReferences } from './clipboard-file-references'
import {
  detectStructuredContent,
  wrapInFence,
} from '../shared/structured-content'
import {
  deriveDocumentName,
  shouldRouteTextToDocument,
} from '../shared/text-document-routing'

const CLIPBOARD_PREFIX_V1 = 'web-canvas:pages:'
export const CLIPBOARD_PREFIX = 'web-canvas:entities:'

function parseClipboardSelection(
  rawText: string,
): ClipboardEntitySelectionPayload | ClipboardPageSelectionPayload | null {
  if (rawText.startsWith(CLIPBOARD_PREFIX)) {
    try {
      const parsed = JSON.parse(
        rawText.slice(CLIPBOARD_PREFIX.length),
      ) as ClipboardEntitySelectionPayload
      if (parsed?.version === 2 && Array.isArray(parsed.entities)) {
        return parsed
      }
    } catch {
      // fall through
    }
  }
  if (rawText.startsWith(CLIPBOARD_PREFIX_V1)) {
    try {
      const parsed = JSON.parse(
        rawText.slice(CLIPBOARD_PREFIX_V1.length),
      ) as ClipboardPageSelectionPayload
      if (parsed?.version === 1 && Array.isArray(parsed.pages)) {
        return parsed
      }
    } catch {
      // fall through
    }
  }
  return null
}

function readClipboardFileReferenceStrings(text: string): string[] {
  const references: string[] = [text]

  try {
    const bookmark = clipboard.readBookmark()
    if (bookmark.url) references.push(bookmark.url)
  } catch {
    // readBookmark is platform-specific.
  }

  let formats: string[] = []
  try {
    formats = clipboard.availableFormats()
  } catch {
    return references
  }

  for (const format of formats) {
    if (
      format === 'public.file-url' ||
      format === 'text/uri-list' ||
      format === 'x-special/gnome-copied-files' ||
      /file-url/i.test(format)
    ) {
      try {
        references.push(clipboard.read(format))
      } catch {
        // Some native formats are advertised but not readable as strings.
      }
    }
  }

  return references
}

function readClipboardStructuredData(text: string): {
  types: string[]
  getData: (type: string) => string
} {
  let formats: string[] = []
  try {
    formats = clipboard.availableFormats()
  } catch {
    formats = []
  }

  const data = new Map<string, string>()
  if (text) data.set('text/plain', text)

  try {
    const html = clipboard.readHTML()
    if (html) data.set('text/html', html)
  } catch {
    // readHTML can be unavailable for some clipboard owners.
  }

  for (const format of formats) {
    if (format === 'image/svg+xml' || format === 'application/json') {
      try {
        const value = clipboard.read(format)
        if (value) data.set(format, value)
      } catch {
        // Some native formats are advertised but not readable as strings.
      }
    }
  }

  return {
    types: Array.from(new Set([...formats, ...data.keys()])),
    getData: (type: string) => data.get(type) ?? '',
  }
}

function createFileEntitiesFromPaths(paths: string[], canvasX: number, canvasY: number): void {
  paths.forEach((file, i) => {
    const dims = imageSizeFromPath(file) ?? videoSizeFromPath(file) ?? htmlDefaultSize(file)
    createFileEntity({
      canvasX: canvasX + i * 20,
      canvasY: canvasY + i * 20,
      file,
      width: dims?.width,
      height: dims?.height,
    })
  })
}

function createTextDocumentEntity(input: {
  canvasX: number
  canvasY: number
  content: string
  name?: string
  width?: number
  height?: number
}): void {
  const file = createNoteFile(input.name ?? deriveDocumentName(input.content), input.content)
  createFileEntity({
    canvasX: input.canvasX,
    canvasY: input.canvasY,
    file,
    width: input.width ?? 400,
    height: input.height ?? 400,
  })
}

function createStructuredContentEntity(input: {
  canvasX: number
  canvasY: number
  lang: string
  text: string
}): boolean {
  if (input.lang === 'svg') {
    const file = saveImageBuffer(Buffer.from(input.text, 'utf8'), 'svg')
    const dims = imageSizeFromPath(file)
    createFileEntity({
      canvasX: input.canvasX,
      canvasY: input.canvasY,
      file,
      width: dims?.width,
      height: dims?.height,
    })
    return true
  }

  if (input.lang === 'html') {
    const file = saveImageBuffer(Buffer.from(input.text, 'utf8'), 'html')
    const dims = htmlDefaultSize(file)
    createFileEntity({
      canvasX: input.canvasX,
      canvasY: input.canvasY,
      file,
      width: dims?.width,
      height: dims?.height,
    })
    return true
  }

  if (input.lang === 'json') {
    createTextDocumentEntity({
      canvasX: input.canvasX,
      canvasY: input.canvasY,
      name: 'JSON',
      content: wrapInFence('json', input.text),
    })
    return true
  }

  return false
}

// Smart-paste resolution order: entity JSON → local file refs → native image
// → structured renderer-backed text → markdown document → URL → sticky text.
// Tested via tests/smoke/keyboard-shortcuts.test.ts.
export function pasteFromClipboard(input: { canvasX: number; canvasY: number }): void {
  const { canvasX, canvasY } = input

  const text = clipboard.readText()
  const payload = parseClipboardSelection(text)
  if (payload) {
    if (payload.version === 2) {
      pasteEntitiesFromClipboard({ payload, canvasX, canvasY })
    } else {
      pastePagesFromClipboard({ payload, canvasX, canvasY })
    }
    return
  }

  const filePaths = filePathsFromClipboardReferences(
    readClipboardFileReferenceStrings(text),
  ).filter((filePath) => existsSync(filePath))
  if (filePaths.length > 0) {
    createFileEntitiesFromPaths(filePaths, canvasX, canvasY)
    return
  }

  const structured = detectStructuredContent(readClipboardStructuredData(text))
  if (
    structured?.lang === 'svg' &&
    createStructuredContentEntity({ canvasX, canvasY, ...structured })
  ) {
    return
  }

  const clipImage = clipboard.readImage()
  if (!clipImage.isEmpty()) {
    const file = saveImageBuffer(clipImage.toPNG(), 'png')
    const { width, height } = clipImage.getSize()
    createFileEntity({ canvasX, canvasY, file, width, height })
    return
  }

  if (structured && createStructuredContentEntity({ canvasX, canvasY, ...structured })) {
    return
  }

  const trimmed = text.trim()
  if (trimmed && shouldRouteTextToDocument(trimmed)) {
    createTextDocumentEntity({ canvasX, canvasY, content: trimmed })
    return
  }

  if (trimmed && !trimmed.includes('\n') && looksLikeUrl(trimmed)) {
    try {
      const url = normalizeUserUrl(trimmed)
      createPageAtPosition({
        presetIndex: DESKTOP_PRESET_INDEX,
        canvasX,
        canvasY,
        mode: 'paste_url',
        focus: true,
        url,
      })
      return
    } catch {
      // Not a valid URL after normalisation — fall through.
    }
  }

  if (trimmed) {
    createTextEntity({
      canvasX,
      canvasY,
      text: trimmed,
      textStyle: 'sticky',
      color: getStickyDefaultColor(),
      textSize: getStickyDefaultSize(),
    })
  }
}
