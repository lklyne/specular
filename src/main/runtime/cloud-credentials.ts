/**
 * Device credential store for the cloud-share owner principal (ADR 0018 §4,
 * tier 1). On first cloud contact the desktop app signs in anonymously and
 * persists the resulting better-auth session here so every later publish/link
 * call reuses one principal instead of minting a new anonymous account.
 *
 * This is a *credential*, so it lives OUTSIDE `preferences.json` in its own
 * file (mode 0600) and never touches a `.canvas` file (tokens are the security
 * boundary — ADR 0018 §4). Keyed by server URL so pointing the dev flag at a
 * different server keeps a distinct session.
 */

import { app } from 'electron'
import { join } from 'path'
import { chmodSync, existsSync, readFileSync, renameSync, writeFileSync } from 'fs'

const CLOUD_CREDENTIALS_FILE = 'cloud-credentials.json'

export interface DeviceSession {
  userId: string
  /** `name=value` cookie header presented on owner-authenticated calls. */
  cookie: string
}

/** `{ [serverUrl]: DeviceSession }`. */
type CredentialsFile = Record<string, DeviceSession>

function credentialsPath(): string {
  return join(app.getPath('userData'), CLOUD_CREDENTIALS_FILE)
}

function readCredentials(): CredentialsFile {
  const file = credentialsPath()
  if (!existsSync(file)) return {}
  try {
    const parsed = JSON.parse(readFileSync(file, 'utf8'))
    return parsed && typeof parsed === 'object' ? (parsed as CredentialsFile) : {}
  } catch {
    return {}
  }
}

function writeCredentials(next: CredentialsFile): void {
  const file = credentialsPath()
  const tmp = `${file}.tmp`
  // Create with 0600, then chmod the tmp in case the umask widened it, so the
  // rename lands a private file (credential, not general config).
  writeFileSync(tmp, JSON.stringify(next, null, 2), { encoding: 'utf8', mode: 0o600 })
  chmodSync(tmp, 0o600)
  renameSync(tmp, file)
}

export function getStoredSession(serverUrl: string): DeviceSession | null {
  const session = readCredentials()[serverUrl]
  return session && typeof session.cookie === 'string' ? session : null
}

export function storeSession(serverUrl: string, session: DeviceSession): void {
  const all = readCredentials()
  all[serverUrl] = session
  writeCredentials(all)
}

/** Drop every stored session (used for test isolation and explicit sign-out). */
export function clearStoredSessions(): void {
  const file = credentialsPath()
  if (existsSync(file)) writeCredentials({})
}
