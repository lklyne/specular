import * as Y from "yjs";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { bootServerHarness, type ServerHarness } from "./harness";
import {
  connectWithToken,
  createLink,
  delay,
  ownedEditDoc,
  redeem,
  upgradeRejected,
  waitFor,
} from "./helpers";

describe("WebSocket upgrade auth", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("rejects a connection with no token", async () => {
    const { docId } = await ownedEditDoc(harness.url);
    expect(await upgradeRejected(harness, docId)).toBe(true);
  });

  it("rejects a connection with an invalid token", async () => {
    const { docId } = await ownedEditDoc(harness.url);
    expect(await upgradeRejected(harness, docId, "garbage")).toBe(true);
  });

  it("rejects a valid token presented for a different doc", async () => {
    const { editToken } = await ownedEditDoc(harness.url);
    const other = await ownedEditDoc(harness.url);
    // editToken is scoped to the first doc; connecting to `other.docId` fails.
    expect(await upgradeRejected(harness, other.docId, editToken)).toBe(true);
  });

  it("accepts a valid edit token and syncs", async () => {
    const { docId, editToken } = await ownedEditDoc(harness.url);
    const doc = new Y.Doc();
    const provider = connectWithToken(harness, docId, doc, editToken);
    await waitFor(() => provider.synced);
    provider.destroy();
  });
});

describe("scope enforcement", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("drops writes from a view connection but lands writes from an edit connection", async () => {
    const owner = await ownedEditDoc(harness.url);
    const { docId } = owner;

    // A guest view connection, derived by redeeming a view link.
    const viewLink = await createLink(
      harness.url,
      owner.principal.cookie,
      docId,
      "view",
    );
    const viewConn = await redeem(harness.url, viewLink.token);
    expect(viewConn.body.scope).toBe("view");

    const editDoc = new Y.Doc();
    const viewDoc = new Y.Doc();
    const editProvider = connectWithToken(harness, docId, editDoc, owner.editToken);
    const viewProvider = connectWithToken(
      harness,
      docId,
      viewDoc,
      viewConn.body.token!,
    );
    await waitFor(() => editProvider.synced && viewProvider.synced);

    // The view peer attempts a write. Its sync-update message is dropped at the
    // DO, so it never reaches the edit peer or the shared doc.
    viewDoc.getMap("canvas").set("fromView", "nope");
    await delay(500);
    expect(editDoc.getMap("canvas").get("fromView")).toBeUndefined();

    // The edit peer's write propagates normally — proving read still works for
    // the view peer and that only scope gates writes.
    editDoc.getMap("canvas").set("fromEdit", "yes");
    await waitFor(() => viewDoc.getMap("canvas").get("fromEdit") === "yes");
    expect(viewDoc.getMap("canvas").get("fromEdit")).toBe("yes");

    editProvider.destroy();
    viewProvider.destroy();
    await delay(700); // let onSave flush.

    // DO state reflects only the edit write, never the dropped view write.
    const reader = new Y.Doc();
    const readerProvider = connectWithToken(harness, docId, reader, owner.editToken);
    await waitFor(() => reader.getMap("canvas").get("fromEdit") === "yes");
    expect(reader.getMap("canvas").get("fromView")).toBeUndefined();
    readerProvider.destroy();
  });
});
