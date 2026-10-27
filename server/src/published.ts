import { desc, eq } from "drizzle-orm";

import { connectionFor, json } from "./connection-auth";
import { getDb, randomToken } from "./db";
import type { Env } from "./env";
import { versions } from "./schema";

/** The manifest format this server takes (`specular-web`'s `FORMAT`). */
const FORMAT = 1;
/** The most a manifest's JSON may be. */
const MAX_MANIFEST_BYTES = 256 * 1024;

/**
 * What a published canvas is: the `.canvas` text, each page's snapshot and
 * each file the canvas names, all as content-addressed asset hashes
 * (`specular-web/src/bundle.rs` is the other side of this).
 */
interface Manifest {
  format: number;
  title: string;
  publishedAt: string;
  canvas: string;
  pages: Record<string, { asset: string; width: number; height: number }>;
  files: Record<string, string>;
}

const HASH = /^[0-9a-f]{64}$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Why `value` is not a manifest this server takes, or `null` when it is. */
function manifestProblem(value: unknown): string | null {
  if (!isRecord(value)) return "not an object";
  if (value.format !== FORMAT) return `format must be ${FORMAT}`;
  if (typeof value.title !== "string" || value.title.length > 200) {
    return "title must be a string of at most 200 characters";
  }
  if (typeof value.publishedAt !== "string") return "publishedAt is required";
  if (typeof value.canvas !== "string" || !HASH.test(value.canvas)) {
    return "canvas must be an asset hash";
  }
  if (!isRecord(value.pages)) return "pages must be an object";
  for (const [id, page] of Object.entries(value.pages)) {
    if (
      !isRecord(page) ||
      typeof page.asset !== "string" ||
      !HASH.test(page.asset) ||
      !Number.isInteger(page.width) ||
      !Number.isInteger(page.height)
    ) {
      return `page ${id} needs an asset hash, a width and a height`;
    }
  }
  if (!isRecord(value.files)) return "files must be an object";
  for (const [path, hash] of Object.entries(value.files)) {
    if (typeof hash !== "string" || !HASH.test(hash)) {
      return `file ${path} must be an asset hash`;
    }
  }
  return null;
}

/** Every asset hash a manifest names, once each. */
function assetsOf(manifest: Manifest): string[] {
  const pages = Object.values(manifest.pages).map((page) => page.asset);
  return [...new Set([manifest.canvas, ...pages, ...Object.values(manifest.files)])];
}

/**
 * POST /docs/:docId/versions — publish a version. Edit scope only. Every
 * asset it names must have been uploaded first, so a link never shows a
 * canvas with holes in it.
 */
export async function publish(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;
  if (resolved.scope !== "edit") return json({ error: "forbidden" }, 403);

  const text = await request.text();
  if (text.length > MAX_MANIFEST_BYTES) {
    return json({ error: "manifest too large" }, 413);
  }
  let body: unknown;
  try {
    body = JSON.parse(text);
  } catch {
    return json({ error: "invalid manifest: not JSON" }, 400);
  }
  const problem = manifestProblem(body);
  if (problem) return json({ error: `invalid manifest: ${problem}` }, 400);
  const manifest = body as Manifest;

  for (const hash of assetsOf(manifest)) {
    if (!(await env.ASSETS.head(`assets/${hash}`))) {
      return json({ error: "missing asset", asset: hash }, 400);
    }
  }

  const versionId = randomToken();
  await getDb(env)
    .insert(versions)
    .values({
      versionId,
      docId,
      manifest: JSON.stringify(manifest),
      createdAt: new Date(),
    });
  return json({ versionId }, 201);
}

/**
 * GET /docs/:docId/manifest — the newest version, with the scope the
 * caller's connection has so the viewer knows whether to offer comments.
 */
export async function getManifest(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;

  const row = await getDb(env)
    .select()
    .from(versions)
    .where(eq(versions.docId, docId))
    .orderBy(desc(versions.createdAt))
    .limit(1)
    .get();
  if (!row) return json({ error: "not published" }, 404);
  return json({
    versionId: row.versionId,
    scope: resolved.scope,
    manifest: JSON.parse(row.manifest) as Manifest,
  });
}
