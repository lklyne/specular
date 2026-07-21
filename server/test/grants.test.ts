import * as Y from "yjs";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { bootServerHarness, type ServerHarness } from "./harness";
import {
  connectWithToken,
  createApiKey,
  createDoc,
  createLink,
  delay,
  ownerConnect,
  redeem,
  signInAnonymous,
  upgradeRejected,
  waitFor,
} from "./helpers";

describe("capability links", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("is idempotent per scope and lists active links", async () => {
    const owner = await signInAnonymous(harness.url);
    const docId = await createDoc(harness.url, { cookie: owner.cookie });

    const first = await createLink(harness.url, owner.cookie, docId, "comment");
    const again = await createLink(harness.url, owner.cookie, docId, "comment");
    // Same scope → same durable grant (same id + token).
    expect(again.grantId).toBe(first.grantId);
    expect(again.token).toBe(first.token);
    expect(first.url).toBe(`/c/${docId}#t=${first.token}`);

    // A different scope is a distinct grant.
    const edit = await createLink(harness.url, owner.cookie, docId, "edit");
    expect(edit.grantId).not.toBe(first.grantId);

    const listRes = await fetch(`${harness.url}/docs/${docId}/links`, {
      headers: { cookie: owner.cookie },
    });
    const { links } = (await listRes.json()) as { links: { scope: string }[] };
    expect(links.map((l) => l.scope).sort()).toEqual(["comment", "edit"]);
  });

  it("rejects link management from a non-owner", async () => {
    const owner = await signInAnonymous(harness.url);
    const docId = await createDoc(harness.url, { cookie: owner.cookie });
    const stranger = await signInAnonymous(harness.url);

    const create = await fetch(`${harness.url}/docs/${docId}/links`, {
      method: "POST",
      headers: { "content-type": "application/json", cookie: stranger.cookie },
      body: JSON.stringify({ scope: "view" }),
    });
    expect(create.status).toBe(403);

    const list = await fetch(`${harness.url}/docs/${docId}/links`, {
      headers: { cookie: stranger.cookie },
    });
    expect(list.status).toBe(403);
  });

  it("rejects a bad scope", async () => {
    const owner = await signInAnonymous(harness.url);
    const docId = await createDoc(harness.url, { cookie: owner.cookie });
    const res = await fetch(`${harness.url}/docs/${docId}/links`, {
      method: "POST",
      headers: { "content-type": "application/json", cookie: owner.cookie },
      body: JSON.stringify({ scope: "admin" }),
    });
    expect(res.status).toBe(400);
  });
});

describe("link redemption", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("mints a connection token from a valid link token", async () => {
    const owner = await signInAnonymous(harness.url);
    const docId = await createDoc(harness.url, { cookie: owner.cookie });
    const link = await createLink(harness.url, owner.cookie, docId, "view");

    const { status, body } = await redeem(harness.url, link.token);
    expect(status).toBe(201);
    expect(body.docId).toBe(docId);
    expect(body.scope).toBe("view");
    expect(typeof body.token).toBe("string");
    expect(body.token).not.toBe(link.token); // derived, not the link itself
    expect(body.expiresAt).toBeGreaterThan(Date.now());
  });

  it("rejects an unknown token", async () => {
    const { status } = await redeem(harness.url, "does-not-exist");
    expect(status).toBe(401);
  });

  it("rejects an expired grant", async () => {
    const owner = await signInAnonymous(harness.url);
    const docId = await createDoc(harness.url, { cookie: owner.cookie });
    const link = await createLink(harness.url, owner.cookie, docId, "view");

    // Backdate the grant's expiry directly in D1 (drizzle timestamp = seconds).
    const db = await harness.mf.getD1Database("DB");
    const pastSeconds = Math.floor(Date.now() / 1000) - 3600;
    await db
      .prepare("UPDATE grants SET expiresAt = ? WHERE grantId = ?")
      .bind(pastSeconds, link.grantId)
      .run();

    const { status } = await redeem(harness.url, link.token);
    expect(status).toBe(401);
  });
});

describe("revoke and reset cut off derived access", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("reset rotates the token, invalidating the old link and its connection tokens", async () => {
    const owner = await signInAnonymous(harness.url);
    const docId = await createDoc(harness.url, { cookie: owner.cookie });
    const link = await createLink(harness.url, owner.cookie, docId, "edit");

    // Derive a connection token from the current generation, prove it connects.
    const conn = await redeem(harness.url, link.token);
    expect(conn.status).toBe(201);
    expect(await upgradeRejected(harness, docId, conn.body.token)).toBe(false);

    // Reset rotates the grant token (same grantId).
    const resetRes = await fetch(
      `${harness.url}/docs/${docId}/links/${link.grantId}/reset`,
      { method: "POST", headers: { cookie: owner.cookie } },
    );
    const rotated = (await resetRes.json()) as { grantId: string; token: string };
    expect(rotated.grantId).toBe(link.grantId);
    expect(rotated.token).not.toBe(link.token);

    // Old link token no longer redeems.
    expect((await redeem(harness.url, link.token)).status).toBe(401);
    // The connection token derived from the old generation is now rejected on
    // new WS connects (its pinned grantToken no longer matches).
    expect(await upgradeRejected(harness, docId, conn.body.token)).toBe(true);
    // A token from the new generation works.
    const fresh = await redeem(harness.url, rotated.token);
    expect(fresh.status).toBe(201);
    expect(await upgradeRejected(harness, docId, fresh.body.token)).toBe(false);
  });

  it("revoke deletes the link and its derived connection tokens", async () => {
    const owner = await signInAnonymous(harness.url);
    const docId = await createDoc(harness.url, { cookie: owner.cookie });
    const link = await createLink(harness.url, owner.cookie, docId, "edit");
    const conn = await redeem(harness.url, link.token);
    expect(await upgradeRejected(harness, docId, conn.body.token)).toBe(false);

    const del = await fetch(
      `${harness.url}/docs/${docId}/links/${link.grantId}`,
      { method: "DELETE", headers: { cookie: owner.cookie } },
    );
    expect(del.status).toBe(200);

    expect((await redeem(harness.url, link.token)).status).toBe(401);
    expect(await upgradeRejected(harness, docId, conn.body.token)).toBe(true);
  });
});

describe("owner shortcut connect", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("mints an edit connection token via cookie and via api key", async () => {
    const owner = await signInAnonymous(harness.url);
    const docId = await createDoc(harness.url, { cookie: owner.cookie });

    const viaCookie = await ownerConnect(harness.url, { cookie: owner.cookie }, docId);
    expect(viaCookie.scope).toBe("edit");

    // The x-api-key owner-auth path resolves to the same principal.
    const apiKey = await createApiKey(harness.url, owner.cookie);
    const viaKey = await ownerConnect(harness.url, { apiKey }, docId);
    expect(viaKey.scope).toBe("edit");

    // Prove a minted edit token actually drives sync end-to-end.
    const doc = new Y.Doc();
    const provider = connectWithToken(harness, docId, doc, viaCookie.token);
    await waitFor(() => provider.synced);
    doc.getMap("canvas").set("ok", true);
    await delay(50);
    provider.destroy();
  });
});
