import { connectionFor, json } from "./connection-auth";
import type { Env } from "./env";

/** The largest asset one upload may carry. */
export const MAX_ASSET_BYTES = 25 * 1024 * 1024;

/**
 * The types an asset is served as. Anything else is kept as
 * `application/octet-stream`, so an upload cannot choose to be served as a
 * page on the viewer's origin.
 */
const SERVED_TYPES = new Set([
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
  "image/avif",
  "image/svg+xml",
  "text/markdown",
  "text/plain",
  "font/woff2",
]);

/**
 * Headers on every asset. A hash names bytes anyone with an edit link wrote,
 * so opened directly they run nothing, and the browser never sniffs a type.
 */
const ASSET_HEADERS: Record<string, string> = {
  "x-content-type-options": "nosniff",
  "content-security-policy": "sandbox; default-src 'none'; img-src data:; style-src 'unsafe-inline'",
  "cache-control": "public, max-age=31536000, immutable",
};

/** The type an upload is stored and served as, from what it claims to be. */
export function servedType(claimed: string | null): string {
  const type = (claimed ?? "").split(";")[0].trim().toLowerCase();
  if (!SERVED_TYPES.has(type)) return "application/octet-stream";
  return type.startsWith("text/") ? `${type}; charset=utf-8` : type;
}

/**
 * POST /docs/:docId/assets — content-addressed upload. Auth is an edit-scope
 * connection token in `Authorization: Bearer <token>`. Body is raw bytes;
 * sha256 → R2 key `assets/<hash>`, deduped (skip put if the object exists).
 */
export async function uploadAsset(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;
  if (resolved.scope !== "edit") return json({ error: "forbidden" }, 403);

  const declared = Number(request.headers.get("content-length") ?? 0);
  if (declared > MAX_ASSET_BYTES) return json({ error: "asset too large" }, 413);
  const bytes = new Uint8Array(await request.arrayBuffer());
  if (bytes.byteLength > MAX_ASSET_BYTES) return json({ error: "asset too large" }, 413);

  const digest = await crypto.subtle.digest("SHA-256", bytes);
  const hash = Array.from(new Uint8Array(digest), (b) =>
    b.toString(16).padStart(2, "0"),
  ).join("");
  const key = `assets/${hash}`;

  // Dedupe: identical bytes hash to the same key, so an existing object is
  // already the correct content — skip the put.
  const head = await env.ASSETS.head(key);
  if (!head) {
    await env.ASSETS.put(key, bytes, {
      httpMetadata: { contentType: servedType(request.headers.get("content-type")) },
    });
  }
  return json({ assetId: hash, size: bytes.byteLength }, 201);
}

/**
 * GET /assets/:hash — serve bytes from R2. Unauthenticated: the
 * content-addressed key is unguessable. The type is re-checked on the way
 * out, so objects stored before the allowlist are served safely too.
 */
export async function getAsset(env: Env, hash: string): Promise<Response> {
  const object = await env.ASSETS.get(`assets/${hash}`);
  if (!object) return new Response("Not found", { status: 404 });

  const headers = new Headers(ASSET_HEADERS);
  headers.set("content-type", servedType(object.httpMetadata?.contentType ?? null));
  headers.set("etag", object.httpEtag);
  return new Response(object.body, { headers });
}
