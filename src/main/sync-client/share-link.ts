/**
 * Share-link parsing + the token/asset HTTP calls a headless peer makes.
 *
 * Pure node (no electron): shared by the `specular connect` CLI verb and the
 * cloud-sync integration test. A share link is the guest credential — it names
 * the doc and carries a grant token that `POST /redeem` exchanges for a
 * short-TTL connection token (the thing the DO's upgrade auth actually checks).
 */

/** The server base URL, the doc id, and the grant token carried by a link. */
export interface ParsedShareLink {
  /** http(s) base the server is reachable at, e.g. `http://localhost:8787`. */
  base: string
  docId: string
  /** Grant token from the link — redeem it for a connection token. */
  token: string
}

export interface ConnectionToken {
  token: string
  docId: string
  scope: string
  expiresAt: number
}

/**
 * Parse a share link. Canonical form is `<base>/c/<docId>#t=<token>` (ADR 0018
 * §4b); the token also travels as a `?t=` query param (raw form), since a
 * fragment is client-only and some transports drop it. Everything before `/c/`
 * is the server base.
 */
export function parseShareLink(link: string): ParsedShareLink {
  const url = new URL(link)
  const marker = '/c/'
  const idx = url.pathname.indexOf(marker)
  if (idx === -1) {
    throw new Error(`not a share link (expected \`/c/<docId>\`): ${link}`)
  }
  const base = `${url.origin}${url.pathname.slice(0, idx)}`.replace(/\/+$/, '')
  const docId = url.pathname.slice(idx + marker.length).replace(/\/+$/, '')
  if (!docId) throw new Error(`share link is missing a doc id: ${link}`)

  const token = tokenFromFragment(url.hash) ?? url.searchParams.get('t') ?? undefined
  if (!token) throw new Error(`share link is missing a token (\`#t=\` or \`?t=\`): ${link}`)

  return { base, docId, token }
}

function tokenFromFragment(hash: string): string | null {
  if (!hash) return null
  const params = new URLSearchParams(hash.replace(/^#/, ''))
  return params.get('t')
}

/**
 * Build a canonical share link (`<base>/c/<docId>#t=<token>`, ADR 0018 §4b) —
 * the inverse of `parseShareLink`. The token rides the fragment so it never
 * reaches the server in a request line. Trailing slashes on the base are
 * dropped so `http://host/` and `http://host` produce the same link.
 */
export function buildShareLink(params: { base: string; docId: string; token: string }): string {
  const base = params.base.replace(/\/+$/, '')
  return `${base}/c/${params.docId}#t=${params.token}`
}

/** Exchange a grant token for a short-TTL connection token (`POST /redeem`). */
export async function redeemLink(base: string, grantToken: string): Promise<ConnectionToken> {
  const res = await fetch(`${base}/redeem`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ token: grantToken }),
  })
  if (!res.ok) {
    throw new Error(`redeem failed (${res.status}): ${await res.text()}`)
  }
  return (await res.json()) as ConnectionToken
}

/**
 * Content-addressed asset upload (`POST /docs/:docId/assets`). Auth is the
 * connection token as a Bearer credential; the server hashes the bytes and
 * returns the sha256 `assetId`. Same bytes dedupe to the same id server-side.
 */
export async function uploadAsset(
  base: string,
  docId: string,
  connToken: string,
  bytes: Uint8Array,
  contentType: string,
): Promise<string> {
  const res = await fetch(`${base}/docs/${docId}/assets`, {
    method: 'POST',
    headers: {
      'content-type': contentType,
      authorization: `Bearer ${connToken}`,
    },
    // Node's fetch accepts a typed array; the DOM `BodyInit` typing omits it.
    body: bytes as unknown as BodyInit,
  })
  if (!res.ok) {
    throw new Error(`asset upload failed (${res.status}): ${await res.text()}`)
  }
  const body = (await res.json()) as { assetId: string }
  return body.assetId
}
