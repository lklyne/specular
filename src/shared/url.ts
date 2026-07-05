const SCHEME_PATTERN = /^[a-z][a-z0-9+.-]*:\/\//i

function isLoopbackHostname(hostname: string): boolean {
  return hostname === 'localhost' || hostname === '::1' || hostname === '[::1]'
}

function isPrivateIpv4(hostname: string): boolean {
  const match = hostname.match(/^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/)
  if (!match) return false

  const octets = match.slice(1).map((part) => Number(part))
  if (octets.some((octet) => octet < 0 || octet > 255)) return false

  const [first, second] = octets
  if (first === 10 || first === 127) return true
  if (first === 192 && second === 168) return true
  if (first === 172 && second >= 16 && second <= 31) return true
  if (first === 169 && second === 254) return true
  return false
}

function isLocalHostname(hostname: string): boolean {
  const normalized = hostname.trim().toLowerCase()
  if (!normalized) return false
  return isLoopbackHostname(normalized) || isPrivateIpv4(normalized) || normalized.endsWith('.local')
}

function inferDefaultScheme(value: string): 'http' | 'https' {
  const candidate = value.trim().replace(/^\/\//, '')
  const hostPortMatch = candidate.match(/^\[([^\]]+)\](?::\d+)?(?:\/|$)/)
  if (hostPortMatch) {
    return isLocalHostname(hostPortMatch[1] ?? '') ? 'http' : 'https'
  }

  const host = candidate.match(/^[^/?#:]+/)?.[0] ?? ''
  return isLocalHostname(host) ? 'http' : 'https'
}

export function normalizeUserUrl(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) throw new Error('URL cannot be empty')

  // Opaque-scheme URLs (data:, blob:) are valid page sources as-is. They lack
  // `://`, so the scheme-inference branch below would prepend `https://` and
  // mangle them into an invalid URL.
  if (/^(data|blob):/i.test(trimmed)) return new URL(trimmed).toString()

  const withScheme = SCHEME_PATTERN.test(trimmed)
    ? trimmed
    : `${inferDefaultScheme(trimmed)}://${trimmed.replace(/^\/\//, '')}`

  return new URL(withScheme).toString()
}

export function resolveAddressInput(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) throw new Error('Address cannot be empty')
  if (looksLikeUrl(trimmed)) return normalizeUserUrl(trimmed)
  return `https://www.google.com/search?q=${encodeURIComponent(trimmed)}`
}

// A trimmed single-line string is treated as a URL when it either has an
// explicit http(s) scheme or looks like a bare host (`host.tld[:port][/…]`,
// `localhost[:port][/…]`). Other schemes (file:, mailto:, javascript:) are
// rejected so pasting them into the canvas doesn't create a page.
const BARE_HOST_PATTERN = /^[^\s:/?#]+(?:\.[^\s:/?#]+)+(?::\d+)?(?:[/?#].*)?$/i
const LOCAL_HOST_PATTERN = /^(?:localhost|\[?::1\]?)(?::\d+)?(?:[/?#].*)?$/i
const ANY_SCHEME_PREFIX = /^[a-z][a-z0-9+.-]*:/i

export function looksLikeUrl(value: string): boolean {
  const trimmed = value.trim()
  if (!trimmed || /\s/.test(trimmed)) return false
  if (/^https?:\/\//i.test(trimmed)) {
    try {
      new URL(trimmed)
      return true
    } catch {
      return false
    }
  }
  if (LOCAL_HOST_PATTERN.test(trimmed)) return true
  if (ANY_SCHEME_PREFIX.test(trimmed)) return false
  return BARE_HOST_PATTERN.test(trimmed)
}
