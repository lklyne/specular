import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MAX_ASSET_BYTES } from "../src/assets";
import { bootServerHarness, type ServerHarness } from "./harness";
import {
  createLink,
  ownedEditDoc,
  redeem,
  uploadAsset,
} from "./helpers";

describe("content-addressed asset upload", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("dedupes identical bytes to one object and round-trips via GET", async () => {
    const { docId, editToken } = await ownedEditDoc(harness.url);
    const bytes = new TextEncoder().encode("<html><body>hi</body></html>");

    const first = await uploadAsset(
      harness.url,
      docId,
      editToken,
      bytes,
      "text/html",
    );
    expect(first.status).toBe(201);
    const firstBody = (await first.json()) as { assetId: string; size: number };
    expect(firstBody.size).toBe(bytes.byteLength);

    const second = await uploadAsset(
      harness.url,
      docId,
      editToken,
      bytes,
      "text/html",
    );
    const secondBody = (await second.json()) as { assetId: string };
    // Same bytes → same content-addressed id.
    expect(secondBody.assetId).toBe(firstBody.assetId);

    // Exactly one R2 object exists under that key.
    const bucket = await harness.mf.getR2Bucket("ASSETS");
    const listed = await bucket.list({ prefix: "assets/" });
    expect(listed.objects.length).toBe(1);
    expect(listed.objects[0].key).toBe(`assets/${firstBody.assetId}`);

    // GET returns the bytes with immutable caching, never as a page.
    const got = await fetch(`${harness.url}/assets/${firstBody.assetId}`);
    expect(got.status).toBe(200);
    expect(got.headers.get("content-type")).toBe("application/octet-stream");
    expect(got.headers.get("content-security-policy")).toContain("sandbox");
    expect(got.headers.get("x-content-type-options")).toBe("nosniff");
    expect(got.headers.get("cache-control")).toContain("immutable");
    const roundTripped = new Uint8Array(await got.arrayBuffer());
    expect(roundTripped).toEqual(bytes);
  });

  it("serves an image as the image it claims to be", async () => {
    const { docId, editToken } = await ownedEditDoc(harness.url);
    const res = await uploadAsset(
      harness.url,
      docId,
      editToken,
      new Uint8Array([0x89, 0x50, 0x4e, 0x47]),
      "IMAGE/PNG",
    );
    const { assetId } = (await res.json()) as { assetId: string };
    const got = await fetch(`${harness.url}/assets/${assetId}`);
    expect(got.headers.get("content-type")).toBe("image/png");
  });

  it("refuses an asset past the size limit", async () => {
    const { docId, editToken } = await ownedEditDoc(harness.url);
    const res = await uploadAsset(
      harness.url,
      docId,
      editToken,
      new Uint8Array(MAX_ASSET_BYTES + 1),
      "image/png",
    );
    expect(res.status).toBe(413);
    const bucket = await harness.mf.getR2Bucket("ASSETS");
    expect((await bucket.list({ prefix: "assets/" })).objects).toHaveLength(0);
  });

  it("returns 404 for an unknown hash", async () => {
    const res = await fetch(`${harness.url}/assets/deadbeef`);
    expect(res.status).toBe(404);
  });

  it("rejects upload from a view-scope connection token", async () => {
    const owner = await ownedEditDoc(harness.url);
    const viewLink = await createLink(
      harness.url,
      owner.principal.cookie,
      owner.docId,
      "view",
    );
    const viewConn = await redeem(harness.url, viewLink.token);

    const res = await uploadAsset(
      harness.url,
      owner.docId,
      viewConn.body.token!,
      new TextEncoder().encode("blocked"),
    );
    expect(res.status).toBe(403);
  });

  it("rejects upload with no/invalid token", async () => {
    const { docId } = await ownedEditDoc(harness.url);
    const noAuth = await fetch(`${harness.url}/docs/${docId}/assets`, {
      method: "POST",
      body: new TextEncoder().encode("x").slice().buffer,
    });
    expect(noAuth.status).toBe(401);
  });
});
