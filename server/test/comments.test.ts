import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { bootServerHarness, type ServerHarness } from "./harness";
import { connect, publishedDoc } from "./helpers";

type Comment = {
  id: string;
  text: string;
  status: string;
  author: string;
  authorName?: string;
  createdAt: string;
  replies: { text: string; authorName?: string }[];
};

describe("comments on a published canvas", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  function call(token: string, path: string, method = "GET", body?: unknown) {
    return fetch(`${harness.url}${path}`, {
      method,
      headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  }

  const annotation = (id: string, text: string) => ({
    id,
    anchor: { type: "canvas", canvasX: 10, canvasY: 20 },
    author: "agent",
    text,
    status: "resolved",
    replies: [{ author: "agent", text: "forged", timestamp: "2020-01-01T00:00:00.000Z" }],
    createdAt: "2020-01-01T00:00:00.000Z",
  });

  async function list(token: string, docId: string): Promise<Comment[]> {
    const res = await call(token, `/docs/${docId}/comments`);
    return ((await res.json()) as { comments: Comment[] }).comments;
  }

  it("a comment link adds a comment that every link and the owner see", async () => {
    const { docId, editToken, comment, view } = await publishedDoc(harness.url);
    const commenter = await connect(harness.url, comment);
    const res = await call(commenter, `/docs/${docId}/comments`, "POST", {
      annotation: annotation("c1", "  why is this blue?  "),
      authorName: "Dana",
    });
    expect(res.status).toBe(201);

    for (const token of [await connect(harness.url, view), editToken]) {
      const [stored] = await list(token, docId);
      // The server decides who, when and the state; the text is trimmed.
      expect(stored).toMatchObject({
        id: "c1",
        text: "why is this blue?",
        author: "user",
        authorName: "Dana",
        status: "pending",
        replies: [],
      });
      expect(stored.createdAt).not.toBe("2020-01-01T00:00:00.000Z");
    }
  });

  it("a view link can read but not comment, reply or resolve", async () => {
    const { docId, comment, view } = await publishedDoc(harness.url);
    const commenter = await connect(harness.url, comment);
    await call(commenter, `/docs/${docId}/comments`, "POST", { annotation: annotation("c1", "hi") });
    const viewer = await connect(harness.url, view);
    const attempts = [
      call(viewer, `/docs/${docId}/comments`, "POST", { annotation: annotation("c2", "hi") }),
      call(viewer, `/docs/${docId}/comments/c1/replies`, "POST", { text: "me too" }),
      call(viewer, `/docs/${docId}/comments/c1`, "PATCH", { status: "resolved" }),
      call(viewer, `/docs/${docId}/comments/c1`, "DELETE"),
    ];
    for (const res of await Promise.all(attempts)) expect(res.status).toBe(403);
    expect((await list(viewer, docId)).length).toBe(1);
  });

  it("replies and resolving land on the comment; only the owner deletes", async () => {
    const { docId, editToken, comment } = await publishedDoc(harness.url);
    const commenter = await connect(harness.url, comment);
    await call(commenter, `/docs/${docId}/comments`, "POST", { annotation: annotation("c1", "hi") });

    const replies = await Promise.all([
      call(commenter, `/docs/${docId}/comments/c1/replies`, "POST", { text: "one", authorName: "Ana" }),
      call(commenter, `/docs/${docId}/comments/c1/replies`, "POST", { text: "two" }),
    ]);
    for (const res of replies) expect(res.status).toBe(200);
    const resolved = await call(commenter, `/docs/${docId}/comments/c1`, "PATCH", { status: "resolved" });
    expect(resolved.status).toBe(200);

    const [stored] = await list(commenter, docId);
    expect(stored.status).toBe("resolved");
    expect(stored.replies.map((reply) => reply.text).sort()).toEqual(["one", "two"]);

    expect((await call(commenter, `/docs/${docId}/comments/c1`, "DELETE")).status).toBe(403);
    expect((await call(editToken, `/docs/${docId}/comments/c1`, "DELETE")).status).toBe(200);
    expect(await list(editToken, docId)).toEqual([]);
  });

  it("refuses a duplicate id, an empty comment and a bad anchor", async () => {
    const { docId, comment } = await publishedDoc(harness.url);
    const commenter = await connect(harness.url, comment);
    const post = (body: unknown) => call(commenter, `/docs/${docId}/comments`, "POST", body);
    expect((await post({ annotation: annotation("c1", "hi") })).status).toBe(201);
    expect((await post({ annotation: annotation("c1", "again") })).status).toBe(409);
    expect((await post({ annotation: annotation("c2", "   ") })).status).toBe(400);
    expect((await post({ annotation: { ...annotation("c3", "hi"), anchor: { type: "x" } } })).status).toBe(400);
    expect((await post({ annotation: annotation("../c4", "hi") })).status).toBe(400);
  });

  it("refuses a token for another doc", async () => {
    const first = await publishedDoc(harness.url);
    const second = await publishedDoc(harness.url);
    const commenter = await connect(harness.url, first.comment);
    const res = await call(commenter, `/docs/${second.docId}/comments`);
    expect(res.status).toBe(401);
  });
});
