import { eq } from "drizzle-orm";

import { getDb, type Db, type Scope } from "./db";
import type { Env } from "./env";
import { connectionTokens, grants } from "./schema";

/**
 * Result of resolving a connection token: the scope the connection is granted
 * and the doc it may join. `null` means reject the upgrade.
 */
export interface ResolvedConnection {
  scope: Scope;
  docId: string;
  userId: string | null;
}

/**
 * Validate a connection token against D1 (ADR 0018 §4). A token is valid iff:
 *  - the row exists and has not expired, and
 *  - if it was derived from a capability link (`parentGrantId` set), the parent
 *    grant still exists AND its current `token` equals the generation the
 *    connection token pinned at mint time (`grantToken`). Rotating (reset) or
 *    deleting (revoke) the grant therefore transitively invalidates every
 *    connection token derived from the old generation.
 *
 * Owner-shortcut tokens carry no parent and pass on TTL alone.
 */
export async function resolveConnectionToken(
  db: Db,
  token: string,
): Promise<ResolvedConnection | null> {
  const row = await db
    .select()
    .from(connectionTokens)
    .where(eq(connectionTokens.token, token))
    .get();
  if (!row) return null;

  if (row.expiresAt.getTime() <= Date.now()) return null;

  if (row.parentGrantId) {
    const grant = await db
      .select()
      .from(grants)
      .where(eq(grants.grantId, row.parentGrantId))
      .get();
    if (!grant) return null; // revoked
    if (grant.token !== row.grantToken) return null; // reset (rotated generation)
    if (grant.expiresAt && grant.expiresAt.getTime() <= Date.now()) return null;
  }

  return { scope: row.scope as Scope, docId: row.docId, userId: row.userId };
}

/**
 * WebSocket-upgrade auth seam (invoked from `onBeforeConnect`). Reads the
 * connection token from `?token=` and requires it to resolve for THIS docId.
 * Returns the resolved scope so the DO can enforce read-only for view/comment
 * connections; `null` rejects the upgrade.
 */
export async function verifyConnectionToken(
  request: Request,
  env: Env,
  docId: string,
): Promise<ResolvedConnection | null> {
  const url = new URL(request.url);
  const token = url.searchParams.get("token");
  if (!token) return null;

  const resolved = await resolveConnectionToken(getDb(env), token);
  if (!resolved) return null;
  if (resolved.docId !== docId) return null;
  return resolved;
}
