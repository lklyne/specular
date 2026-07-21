import { and, eq } from "drizzle-orm";

import { createAuth } from "./auth";
import { resolveConnectionToken } from "./connection-auth";
import {
  getDb,
  isScope,
  randomToken,
  type Db,
  type Scope,
} from "./db";
import type { Env } from "./env";
import { connectionTokens, docs, grants } from "./schema";

/** Short TTL for connection tokens — they are cheap to re-mint (ADR 0018 §4). */
const CONNECTION_TOKEN_TTL_MS = 60 * 60 * 1000; // ~1h

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

/**
 * Resolve the calling principal from a better-auth session cookie OR an
 * `x-api-key` header (the `enableSessionForAPIKeys` path). Returns the user id
 * or `null` when unauthenticated.
 */
async function principalId(request: Request, env: Env): Promise<string | null> {
  const session = await createAuth(env).api.getSession({
    headers: request.headers,
  });
  return session?.user?.id ?? null;
}

/** Load a doc row and confirm the caller owns it. */
async function ownedDoc(
  db: Db,
  docId: string,
  userId: string,
): Promise<boolean> {
  const row = await db.select().from(docs).where(eq(docs.docId, docId)).get();
  return !!row && row.ownerId === userId;
}

async function mintConnectionToken(
  db: Db,
  params: {
    docId: string;
    scope: Scope;
    parentGrantId?: string;
    grantToken?: string;
    userId?: string | null;
  },
): Promise<{ token: string; docId: string; scope: Scope; expiresAt: number }> {
  const token = randomToken();
  const expiresAt = new Date(Date.now() + CONNECTION_TOKEN_TTL_MS);
  await db.insert(connectionTokens).values({
    token,
    docId: params.docId,
    scope: params.scope,
    parentGrantId: params.parentGrantId ?? null,
    grantToken: params.grantToken ?? null,
    userId: params.userId ?? null,
    expiresAt,
  });
  return {
    token,
    docId: params.docId,
    scope: params.scope,
    expiresAt: expiresAt.getTime(),
  };
}

function linkUrl(docId: string, token: string): string {
  // No web client exists yet; echo the path shape from ADR 0018 §4.
  return `/c/${docId}#t=${token}`;
}

/** POST /docs — mint a doc owned by the caller. */
async function createDoc(request: Request, env: Env): Promise<Response> {
  const userId = await principalId(request, env);
  if (!userId) return json({ error: "unauthorized" }, 401);

  const db = getDb(env);
  const docId = randomToken();
  await db.insert(docs).values({ docId, ownerId: userId, createdAt: new Date() });
  return json({ docId }, 201);
}

/** POST /docs/:docId/links — idempotent per scope. */
async function createLink(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const userId = await principalId(request, env);
  if (!userId) return json({ error: "unauthorized" }, 401);

  const db = getDb(env);
  if (!(await ownedDoc(db, docId, userId))) return json({ error: "forbidden" }, 403);

  const body = (await request.json().catch(() => ({}))) as { scope?: unknown };
  if (!isScope(body.scope)) return json({ error: "invalid scope" }, 400);
  const scope = body.scope;

  const existing = await db
    .select()
    .from(grants)
    .where(and(eq(grants.docId, docId), eq(grants.scope, scope)))
    .get();
  if (existing) {
    return json({
      grantId: existing.grantId,
      scope,
      token: existing.token,
      url: linkUrl(docId, existing.token),
    });
  }

  const grantId = randomToken();
  const token = randomToken();
  await db.insert(grants).values({
    grantId,
    docId,
    scope,
    token,
    createdBy: userId,
    createdAt: new Date(),
  });
  return json({ grantId, scope, token, url: linkUrl(docId, token) }, 201);
}

/** GET /docs/:docId/links — active links for the canvas. */
async function listLinks(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const userId = await principalId(request, env);
  if (!userId) return json({ error: "unauthorized" }, 401);

  const db = getDb(env);
  if (!(await ownedDoc(db, docId, userId))) return json({ error: "forbidden" }, 403);

  const rows = await db.select().from(grants).where(eq(grants.docId, docId)).all();
  return json({
    links: rows.map((r) => ({
      grantId: r.grantId,
      scope: r.scope,
      token: r.token,
      url: linkUrl(docId, r.token),
    })),
  });
}

/** POST /docs/:docId/links/:grantId/reset — rotate the token in place. */
async function resetLink(
  request: Request,
  env: Env,
  docId: string,
  grantId: string,
): Promise<Response> {
  const userId = await principalId(request, env);
  if (!userId) return json({ error: "unauthorized" }, 401);

  const db = getDb(env);
  if (!(await ownedDoc(db, docId, userId))) return json({ error: "forbidden" }, 403);

  const grant = await db
    .select()
    .from(grants)
    .where(and(eq(grants.grantId, grantId), eq(grants.docId, docId)))
    .get();
  if (!grant) return json({ error: "not found" }, 404);

  const token = randomToken();
  await db.update(grants).set({ token }).where(eq(grants.grantId, grantId));
  return json({
    grantId,
    scope: grant.scope,
    token,
    url: linkUrl(docId, token),
  });
}

/** DELETE /docs/:docId/links/:grantId — revoke the link. */
async function revokeLink(
  request: Request,
  env: Env,
  docId: string,
  grantId: string,
): Promise<Response> {
  const userId = await principalId(request, env);
  if (!userId) return json({ error: "unauthorized" }, 401);

  const db = getDb(env);
  if (!(await ownedDoc(db, docId, userId))) return json({ error: "forbidden" }, 403);

  const deleted = await db
    .delete(grants)
    .where(and(eq(grants.grantId, grantId), eq(grants.docId, docId)))
    .returning();
  if (deleted.length === 0) return json({ error: "not found" }, 404);
  return json({ revoked: grantId });
}

/**
 * POST /redeem — exchange a capability-link token for a short-TTL connection
 * token. No owner auth: the link IS the credential (ADR 0018 §4, tier 2/3).
 */
async function redeem(request: Request, env: Env): Promise<Response> {
  const body = (await request.json().catch(() => ({}))) as { token?: unknown };
  if (typeof body.token !== "string") return json({ error: "invalid token" }, 400);

  const db = getDb(env);
  const grant = await db
    .select()
    .from(grants)
    .where(eq(grants.token, body.token))
    .get();
  if (!grant) return json({ error: "invalid token" }, 401);
  if (grant.expiresAt && grant.expiresAt.getTime() <= Date.now()) {
    return json({ error: "expired" }, 401);
  }

  const minted = await mintConnectionToken(db, {
    docId: grant.docId,
    scope: grant.scope as Scope,
    parentGrantId: grant.grantId,
    grantToken: grant.token,
  });
  return json(minted, 201);
}

/**
 * POST /docs/:docId/connect — owner shortcut. Mints an edit-scope connection
 * token directly, no link involved (ADR 0018 §4, "owner's agents" door).
 */
async function ownerConnect(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const userId = await principalId(request, env);
  if (!userId) return json({ error: "unauthorized" }, 401);

  const db = getDb(env);
  if (!(await ownedDoc(db, docId, userId))) return json({ error: "forbidden" }, 403);

  const minted = await mintConnectionToken(db, {
    docId,
    scope: "edit",
    userId,
  });
  return json(minted, 201);
}

/**
 * POST /docs/:docId/assets — content-addressed upload. Auth is an edit-scope
 * connection token in `Authorization: Bearer <token>`. Body is raw bytes;
 * sha256 → R2 key `assets/<hash>`, deduped (skip put if the object exists).
 */
async function uploadAsset(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const auth = request.headers.get("authorization") ?? "";
  const token = auth.toLowerCase().startsWith("bearer ")
    ? auth.slice(7).trim()
    : null;
  if (!token) return json({ error: "unauthorized" }, 401);

  const resolved = await resolveConnectionToken(getDb(env), token);
  if (!resolved || resolved.docId !== docId) {
    return json({ error: "unauthorized" }, 401);
  }
  if (resolved.scope !== "edit") return json({ error: "forbidden" }, 403);

  const bytes = new Uint8Array(await request.arrayBuffer());
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  const hash = Array.from(new Uint8Array(digest), (b) =>
    b.toString(16).padStart(2, "0"),
  ).join("");
  const key = `assets/${hash}`;
  const contentType = request.headers.get("content-type") ?? undefined;

  // Dedupe: identical bytes hash to the same key, so an existing object is
  // already the correct content — skip the put.
  const head = await env.ASSETS.head(key);
  if (!head) {
    await env.ASSETS.put(key, bytes, {
      httpMetadata: contentType ? { contentType } : undefined,
    });
  }
  return json({ assetId: hash, size: bytes.byteLength }, 201);
}

/**
 * GET /assets/:hash — serve bytes from R2. Unauthenticated by design for the
 * spike: the content-addressed key is unguessable, and ADR 0018 §2's
 * sandbox-origin separation for active content is future work. Immutable cache
 * headers since a hash names exactly one immutable object.
 */
async function getAsset(env: Env, hash: string): Promise<Response> {
  const object = await env.ASSETS.get(`assets/${hash}`);
  if (!object) return new Response("Not found", { status: 404 });

  const headers = new Headers();
  object.writeHttpMetadata(headers);
  headers.set("cache-control", "public, max-age=31536000, immutable");
  headers.set("etag", object.httpEtag);
  return new Response(object.body, { headers });
}

/**
 * Dispatch the cloud-sync HTTP surface. Returns `null` when the path is not one
 * of these routes so the worker can fall through to partykit / 404.
 */
export async function handleApiRequest(
  request: Request,
  env: Env,
): Promise<Response | null> {
  const url = new URL(request.url);
  const segments = url.pathname.split("/").filter(Boolean);
  const method = request.method;

  // /assets/:hash
  if (segments[0] === "assets" && segments.length === 2) {
    if (method === "GET") return getAsset(env, segments[1]);
    return json({ error: "method not allowed" }, 405);
  }

  if (segments[0] === "redeem" && segments.length === 1) {
    if (method === "POST") return redeem(request, env);
    return json({ error: "method not allowed" }, 405);
  }

  if (segments[0] === "docs") {
    // POST /docs
    if (segments.length === 1) {
      if (method === "POST") return createDoc(request, env);
      return json({ error: "method not allowed" }, 405);
    }

    const docId = segments[1];

    // /docs/:docId/links[...]
    if (segments[2] === "links") {
      if (segments.length === 3) {
        if (method === "POST") return createLink(request, env, docId);
        if (method === "GET") return listLinks(request, env, docId);
        return json({ error: "method not allowed" }, 405);
      }
      const grantId = segments[3];
      if (segments.length === 4 && method === "DELETE") {
        return revokeLink(request, env, docId, grantId);
      }
      if (segments.length === 5 && segments[4] === "reset" && method === "POST") {
        return resetLink(request, env, docId, grantId);
      }
      return json({ error: "not found" }, 404);
    }

    if (segments[2] === "connect" && segments.length === 3) {
      if (method === "POST") return ownerConnect(request, env, docId);
      return json({ error: "method not allowed" }, 405);
    }

    if (segments[2] === "assets" && segments.length === 3) {
      if (method === "POST") return uploadAsset(request, env, docId);
      return json({ error: "method not allowed" }, 405);
    }
  }

  return null;
}
