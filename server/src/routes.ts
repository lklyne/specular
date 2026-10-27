import { and, eq } from "drizzle-orm";

import { createAuth } from "./auth";
import {
  addReply,
  createComment,
  deleteComment,
  listComments,
  setStatus,
} from "./comments";
import { connectionFor, json } from "./connection-auth";
import {
  getDb,
  isScope,
  randomToken,
  type Db,
  type Scope,
} from "./db";
import type { Env } from "./env";
import { getManifest, publish } from "./published";
import { connectionTokens, docs, grants } from "./schema";

/** Short TTL for connection tokens — they are cheap to re-mint (ADR 0018 §4). */
const CONNECTION_TOKEN_TTL_MS = 60 * 60 * 1000; // ~1h

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

/**
 * The web viewer's address for a link. The token rides in the fragment, so
 * it never reaches a server log or a Referer header (ADR 0018 §4).
 */
function linkUrl(docId: string, token: string): string {
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
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;
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

type Handler = (
  request: Request,
  env: Env,
  params: string[],
) => Promise<Response>;

/**
 * The HTTP surface. A `:` segment matches any one path segment and is passed
 * to the handler, in order, as `params`.
 */
const ROUTES: [string, Partial<Record<string, Handler>>][] = [
  ["assets/:hash", {
    GET: (_, env, [hash]) => getAsset(env, hash),
    HEAD: (_, env, [hash]) => getAsset(env, hash),
  }],
  ["redeem", { POST: (request, env) => redeem(request, env) }],
  ["docs", { POST: (request, env) => createDoc(request, env) }],
  ["docs/:doc/links", {
    POST: (request, env, [doc]) => createLink(request, env, doc),
    GET: (request, env, [doc]) => listLinks(request, env, doc),
  }],
  ["docs/:doc/links/:grant", {
    DELETE: (request, env, [doc, grant]) => revokeLink(request, env, doc, grant),
  }],
  ["docs/:doc/links/:grant/reset", {
    POST: (request, env, [doc, grant]) => resetLink(request, env, doc, grant),
  }],
  ["docs/:doc/connect", { POST: (request, env, [doc]) => ownerConnect(request, env, doc) }],
  ["docs/:doc/assets", { POST: (request, env, [doc]) => uploadAsset(request, env, doc) }],
  ["docs/:doc/versions", { POST: (request, env, [doc]) => publish(request, env, doc) }],
  ["docs/:doc/manifest", { GET: (request, env, [doc]) => getManifest(request, env, doc) }],
  ["docs/:doc/comments", {
    GET: (request, env, [doc]) => listComments(request, env, doc),
    POST: (request, env, [doc]) => createComment(request, env, doc),
  }],
  ["docs/:doc/comments/:id", {
    PATCH: (request, env, [doc, id]) => setStatus(request, env, doc, id),
    DELETE: (request, env, [doc, id]) => deleteComment(request, env, doc, id),
  }],
  ["docs/:doc/comments/:id/replies", {
    POST: (request, env, [doc, id]) => addReply(request, env, doc, id),
  }],
];

/** The `:` segments of `segments` when they fit `pattern`, else `null`. */
function match(pattern: string, segments: string[]): string[] | null {
  const parts = pattern.split("/");
  if (parts.length !== segments.length) return null;
  const params: string[] = [];
  for (const [i, part] of parts.entries()) {
    if (part.startsWith(":")) params.push(segments[i]);
    else if (part !== segments[i]) return null;
  }
  return params;
}

/**
 * Dispatch the HTTP surface. Returns `null` when the path is not one
 * of these routes so the worker can fall through to the web viewer.
 */
export async function handleApiRequest(
  request: Request,
  env: Env,
): Promise<Response | null> {
  const segments = new URL(request.url).pathname.split("/").filter(Boolean);
  for (const [pattern, methods] of ROUTES) {
    const params = match(pattern, segments);
    if (!params) continue;
    const handler = methods[request.method];
    if (!handler) return json({ error: "method not allowed" }, 405);
    return handler(request, env, params);
  }
  return null;
}
