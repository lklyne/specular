import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { bootServerHarness, type ServerHarness } from "./harness";
import {
  CANVAS,
  connect,
  manifest,
  ownedEditDoc,
  publishedDoc,
  uploadText,
} from "./helpers";

describe("publishing a canvas", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  async function publish(docId: string, token: string, body: unknown): Promise<Response> {
    return fetch(`${harness.url}/docs/${docId}/versions`, {
      method: "POST",
      headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
      body: JSON.stringify(body),
    });
  }

  it("serves the newest version to a link, with the link's scope", async () => {
    const { docId, editToken, view } = await publishedDoc(harness.url);
    const canvas = await uploadText(harness.url, docId, editToken, CANVAS + " ");
    expect((await publish(docId, editToken, manifest(canvas, { title: "v2" }))).status).toBe(201);

    const token = await connect(harness.url, view);
    const res = await fetch(`${harness.url}/docs/${docId}/manifest`, {
      headers: { authorization: `Bearer ${token}` },
    });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { scope: string; manifest: { title: string; canvas: string } };
    expect(body.scope).toBe("view");
    expect(body.manifest.title).toBe("v2");
    expect(body.manifest.canvas).toBe(canvas);
  });

  it("refuses a manifest that names an asset never uploaded", async () => {
    const { docId, editToken } = await ownedEditDoc(harness.url);
    const canvas = await uploadText(harness.url, docId, editToken, CANVAS);
    const missing = "0".repeat(64);
    const res = await publish(docId, editToken, manifest(canvas, {
      pages: { p1: { asset: missing, width: 400, height: 300 } },
    }));
    expect(res.status).toBe(400);
    expect(((await res.json()) as { asset: string }).asset).toBe(missing);
  });

  it("refuses a malformed manifest, and a publish without edit scope", async () => {
    const { docId, editToken, comment } = await publishedDoc(harness.url);
    expect((await publish(docId, editToken, { format: 2 })).status).toBe(400);
    const canvas = await uploadText(harness.url, docId, editToken, CANVAS);
    const commenter = await connect(harness.url, comment);
    expect((await publish(docId, commenter, manifest(canvas))).status).toBe(403);
  });

  it("answers 404 for a doc with nothing published", async () => {
    const { docId, editToken } = await ownedEditDoc(harness.url);
    const res = await fetch(`${harness.url}/docs/${docId}/manifest`, {
      headers: { authorization: `Bearer ${editToken}` },
    });
    expect(res.status).toBe(404);
  });
});
