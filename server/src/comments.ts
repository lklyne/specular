import { and, asc, eq } from "drizzle-orm";

import { connectionFor, json, type ResolvedConnection } from "./connection-auth";
import { getDb } from "./db";
import type { Env } from "./env";
import { comments } from "./schema";

/**
 * Comments on a published canvas. Each is stored as the annotation a
 * `.canvas` file holds (`specular-doc/src/annotation.rs`), plus two fields
 * the desktop keeps as unmodeled ones: `authorName`, and `via: "web"`.
 * A comment link can add comments, reply and resolve; only the owner's edit
 * connection can delete.
 */

const MAX_TEXT = 10_000;
const MAX_NAME = 80;
const MAX_REPLIES = 500;
const MAX_BODY_BYTES = 64 * 1024;
const ID = /^[A-Za-z0-9_-]{1,64}$/;
const ANCHORS = new Set(["canvas", "page", "element", "region"]);
const STATUSES = new Set(["pending", "acknowledged", "resolved", "dismissed"]);

type Annotation = Record<string, unknown> & {
  id: string;
  status: string;
  replies: Record<string, unknown>[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function canComment(connection: ResolvedConnection): boolean {
  return connection.scope === "comment" || connection.scope === "edit";
}

/** A display name, trimmed and bounded, or `undefined` for none. */
function nameOf(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const name = value.trim().slice(0, MAX_NAME);
  return name || undefined;
}

/** The text of a comment or reply, or `null` when it is empty or too long. */
function textOf(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const text = value.trim();
  if (!text || text.length > MAX_TEXT) return null;
  return text;
}

async function readJson(request: Request): Promise<unknown> {
  return request.json().catch(() => null);
}

/** GET /docs/:docId/comments — every comment, oldest first. Any scope. */
export async function listComments(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;
  const rows = await getDb(env)
    .select()
    .from(comments)
    .where(eq(comments.docId, docId))
    .orderBy(asc(comments.createdAt))
    .all();
  return json({ comments: rows.map((row) => JSON.parse(row.body) as Annotation) });
}

/**
 * POST /docs/:docId/comments — `{ annotation, authorName }`. The viewer
 * makes the annotation (its id, anchor and text); the server stamps who,
 * when and the starting state, so a client cannot backdate or pre-resolve.
 */
export async function createComment(
  request: Request,
  env: Env,
  docId: string,
): Promise<Response> {
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;
  if (!canComment(resolved)) return json({ error: "forbidden" }, 403);

  const body = await readJson(request);
  const annotation = isRecord(body) ? body.annotation : null;
  if (!isRecord(annotation)) return json({ error: "annotation is required" }, 400);
  if (typeof annotation.id !== "string" || !ID.test(annotation.id)) {
    return json({ error: "invalid id" }, 400);
  }
  const anchor = annotation.anchor;
  if (!isRecord(anchor) || typeof anchor.type !== "string" || !ANCHORS.has(anchor.type)) {
    return json({ error: "invalid anchor" }, 400);
  }
  const text = textOf(annotation.text);
  if (text === null) return json({ error: "text is required" }, 400);

  const now = new Date();
  const stored: Annotation = {
    ...annotation,
    id: annotation.id,
    text,
    author: "user",
    status: "pending",
    replies: [],
    createdAt: now.toISOString(),
    via: "web",
  };
  const authorName = nameOf(isRecord(body) ? body.authorName : undefined);
  if (authorName) stored.authorName = authorName;
  else delete stored.authorName;

  const encoded = JSON.stringify(stored);
  if (encoded.length > MAX_BODY_BYTES) return json({ error: "comment too large" }, 413);

  const db = getDb(env);
  const existing = await db
    .select({ id: comments.id })
    .from(comments)
    .where(and(eq(comments.docId, docId), eq(comments.id, annotation.id)))
    .get();
  if (existing) return json({ error: "a comment with that id exists" }, 409);
  await db.insert(comments).values({
    docId,
    id: annotation.id,
    body: encoded,
    createdAt: now,
    updatedAt: now,
  });
  return json({ comment: stored }, 201);
}

/**
 * Changes one comment with `change`, which returns an error response to
 * refuse. A concurrent change between the read and the write is retried,
 * so two replies at once both land.
 */
async function changeComment(
  env: Env,
  docId: string,
  id: string,
  change: (annotation: Annotation) => Response | null,
): Promise<Response> {
  const db = getDb(env);
  for (let attempt = 0; attempt < 5; attempt++) {
    const row = await db
      .select()
      .from(comments)
      .where(and(eq(comments.docId, docId), eq(comments.id, id)))
      .get();
    if (!row) return json({ error: "not found" }, 404);
    const annotation = JSON.parse(row.body) as Annotation;
    const refused = change(annotation);
    if (refused) return refused;
    const encoded = JSON.stringify(annotation);
    if (encoded.length > MAX_BODY_BYTES) return json({ error: "comment too large" }, 413);
    const updated = await db
      .update(comments)
      .set({ body: encoded, updatedAt: new Date(Math.max(Date.now(), row.updatedAt.getTime() + 1)) })
      .where(
        and(
          eq(comments.docId, docId),
          eq(comments.id, id),
          eq(comments.updatedAt, row.updatedAt),
        ),
      )
      .returning({ id: comments.id });
    if (updated.length > 0) return json({ comment: annotation });
  }
  return json({ error: "the comment kept changing; try again" }, 409);
}

/** POST /docs/:docId/comments/:id/replies — `{ text, authorName }`. */
export async function addReply(
  request: Request,
  env: Env,
  docId: string,
  id: string,
): Promise<Response> {
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;
  if (!canComment(resolved)) return json({ error: "forbidden" }, 403);

  const body = await readJson(request);
  const text = textOf(isRecord(body) ? body.text : undefined);
  if (text === null) return json({ error: "text is required" }, 400);
  const authorName = nameOf(isRecord(body) ? body.authorName : undefined);

  return changeComment(env, docId, id, (annotation) => {
    if (annotation.replies.length >= MAX_REPLIES) {
      return json({ error: "too many replies" }, 413);
    }
    annotation.replies.push({
      author: "user",
      text,
      timestamp: new Date().toISOString(),
      ...(authorName ? { authorName } : {}),
    });
    return null;
  });
}

/** PATCH /docs/:docId/comments/:id — `{ status }`: resolve or reopen. */
export async function setStatus(
  request: Request,
  env: Env,
  docId: string,
  id: string,
): Promise<Response> {
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;
  if (!canComment(resolved)) return json({ error: "forbidden" }, 403);

  const body = await readJson(request);
  const status = isRecord(body) ? body.status : undefined;
  if (typeof status !== "string" || !STATUSES.has(status)) {
    return json({ error: "invalid status" }, 400);
  }
  return changeComment(env, docId, id, (annotation) => {
    annotation.status = status;
    return null;
  });
}

/** DELETE /docs/:docId/comments/:id — the owner's edit connection only. */
export async function deleteComment(
  request: Request,
  env: Env,
  docId: string,
  id: string,
): Promise<Response> {
  const resolved = await connectionFor(request, env, docId);
  if (resolved instanceof Response) return resolved;
  if (resolved.scope !== "edit") return json({ error: "forbidden" }, 403);
  const deleted = await getDb(env)
    .delete(comments)
    .where(and(eq(comments.docId, docId), eq(comments.id, id)))
    .returning({ id: comments.id });
  if (deleted.length === 0) return json({ error: "not found" }, 404);
  return json({ deleted: id });
}
