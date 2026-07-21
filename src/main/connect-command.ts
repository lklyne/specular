/**
 * `specular connect` — the headless cloud-peer verb (cloud-sync spike step 6).
 *
 * Unlike every other verb, this one talks to the CLOUD server directly, not to
 * a running desktop app: it redeems a share link, joins the canvas Durable
 * Object as its own Yjs peer, and writes HTML file entities into the shared
 * doc. The write is the HTML prototyping loop (ADR 0018 §5) — upload bytes,
 * point a file entity at the content hash, let the DO propagate to every peer.
 */

import { readFile } from 'node:fs/promises'
import { printJson, printError } from './cli-output'
import type { ParsedArgs } from './cli-parser'
import { SyncClientSession, type HtmlEntityPlacement } from './sync-client'

const USAGE =
  'usage: specular connect <link> --html <path> [--x N --y N --width N --height N] [--id <entityId>]\n' +
  '       specular connect <link> --status'

function numberFlag(value: string | undefined): number | undefined {
  if (value === undefined) return undefined
  const n = Number(value)
  return Number.isFinite(n) ? n : undefined
}

export async function connectCommand(args: ParsedArgs): Promise<number> {
  const link = args.positional[0]
  if (!link) {
    printError(USAGE)
    return 1
  }

  // --status: cheap liveness read — join, print the doc's per-kind counts, exit.
  if (args.boolFlags.has('status')) {
    const session = await SyncClientSession.join(link)
    try {
      printJson({ docId: session.docId, ...session.summary() })
    } finally {
      session.close()
    }
    return 0
  }

  const htmlPath = args.flags.html
  if (!htmlPath) {
    printError(USAGE)
    return 1
  }

  const html = await readFile(htmlPath)
  const place: HtmlEntityPlacement = {
    canvasX: numberFlag(args.flags.x),
    canvasY: numberFlag(args.flags.y),
    width: numberFlag(args.flags.width),
    height: numberFlag(args.flags.height),
    id: args.flags.id,
  }

  const session = await SyncClientSession.join(link)
  try {
    const result = await session.writeHtmlEntity(html, place)
    printJson(result)
  } finally {
    session.close()
  }
  return 0
}
