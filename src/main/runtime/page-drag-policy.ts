/**
 * What a drag out of a page is allowed to pull in, and how its payload is
 * decoded. Pure decisions, kept apart from `page-drag-out.ts` so they can be
 * read and tested without the runtime around them.
 */

import { isHttpOrFileUrl } from '../../shared/url'

function isFileUrl(value: string): boolean {
  try {
    return new URL(value).protocol === 'file:'
  } catch {
    return false
  }
}

/**
 * Whether a drag out of `sourcePageUrl` may pull `url` into the workspace.
 *
 * A local page dragging a local file is the user handing over their own
 * content. A remote page doing it is not: main fetches what the drag names
 * with its own privileges, so `<img src="file:///Users/…/.ssh/id_rsa">`
 * positioned under the pointer would otherwise save that file into the space
 * on release. Chromium draws the same line for what a document may load.
 */
export function dragMayLoad(url: string, sourcePageUrl: string | undefined): boolean {
  if (!isHttpOrFileUrl(url)) return false
  if (!isFileUrl(url)) return true
  return sourcePageUrl !== undefined && isFileUrl(sourcePageUrl)
}

/**
 * Percent-decode a data URL's body, passing a `%` that begins no escape
 * through as itself. `decodeURIComponent` throws on one, and browsers accept
 * it — inline SVG, which is most of what arrives un-base64'd, is full of
 * `width="100%"`. Decoding to bytes rather than to a string keeps multi-byte
 * UTF-8 escapes intact.
 */
export function decodePercentEncoded(text: string): Buffer {
  const parts: Buffer[] = []
  let literalStart = 0
  for (let i = 0; i < text.length; i++) {
    if (text[i] !== '%') continue
    const hex = text.slice(i + 1, i + 3)
    if (!/^[0-9a-fA-F]{2}$/.test(hex)) continue
    parts.push(Buffer.from(text.slice(literalStart, i), 'utf-8'), Buffer.of(parseInt(hex, 16)))
    i += 2
    literalStart = i + 1
  }
  parts.push(Buffer.from(text.slice(literalStart), 'utf-8'))
  return Buffer.concat(parts)
}
