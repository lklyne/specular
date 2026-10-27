import { eq } from "drizzle-orm";

import { getDb, type Db, type Scope } from "./db";
import type { Env } from "./env";
import { connectionTokens, grants } from "./schema";

/**
 * Result of resolving a connection token: the scope the connection is granted
 * and the doc it may act on. `null` means reject the request.
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
async function resolveConnectionToken(
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

export function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

/**
 * The connection a request presents in `Authorization: Bearer <token>`, if
 * it resolves for THIS docId; otherwise the 401 to answer with.
 */
export async function connectionFor(
  request: Request,
  env: Env,
  docId: string,
): Promise<ResolvedConnection | Response> {
  const auth = request.headers.get("authorization") ?? "";
  const token = auth.toLowerCase().startsWith("bearer ")
    ? auth.slice(7).trim()
    : null;
  if (!token) return json({ error: "unauthorized" }, 401);
  const resolved = await resolveConnectionToken(getDb(env), token);
  if (!resolved || resolved.docId !== docId) {
    return json({ error: "unauthorized" }, 401);
  }
  return resolved;
}
