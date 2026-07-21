import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import * as Y from "yjs";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { bootServerHarness, type ServerHarness } from "./harness";
import {
  connectWithToken,
  delay,
  ownedEditDoc,
  ownerConnect,
  signInAnonymous,
  createDoc,
  waitFor,
  type Principal,
} from "./helpers";

describe("CanvasDoc sync", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("converges two headless Yjs clients through the DO", async () => {
    // Both clients present edit-scope connection tokens for the same doc; the
    // owner mints two independent tokens against one doc.
    const { principal, docId, editToken } = await ownedEditDoc(harness.url);
    const second = await ownerConnect(
      harness.url,
      { cookie: (principal as Principal).cookie },
      docId,
    );

    const docA = new Y.Doc();
    const docB = new Y.Doc();
    const providerA = connectWithToken(harness, docId, docA, editToken);
    const providerB = connectWithToken(harness, docId, docB, second.token);

    await waitFor(() => providerA.synced && providerB.synced);

    docA.getMap("canvas").set("title", "from-A");
    docB.getMap("canvas").set("author", "from-B");

    await waitFor(
      () =>
        docA.getMap("canvas").get("author") === "from-B" &&
        docB.getMap("canvas").get("title") === "from-A",
    );

    expect(docA.getMap("canvas").get("title")).toBe("from-A");
    expect(docA.getMap("canvas").get("author")).toBe("from-B");
    expect(docB.getMap("canvas").get("title")).toBe("from-A");
    expect(docB.getMap("canvas").get("author")).toBe("from-B");

    providerA.destroy();
    providerB.destroy();
  });

  it("loads persisted state for a fresh client after all disconnect", async () => {
    const { docId, editToken } = await ownedEditDoc(harness.url);
    const writer = new Y.Doc();
    const writerProvider = connectWithToken(harness, docId, writer, editToken);
    await waitFor(() => writerProvider.synced);

    writer.getMap("canvas").set("kept", "value");
    // Let the DO's onSave debounce (200ms) flush to storage, then disconnect.
    await delay(700);
    writerProvider.destroy();
    await delay(200);

    const reader = new Y.Doc();
    const readerProvider = connectWithToken(harness, docId, reader, editToken);
    await waitFor(() => reader.getMap("canvas").get("kept") === "value");

    expect(reader.getMap("canvas").get("kept")).toBe("value");
    readerProvider.destroy();
  });
});

describe("CanvasDoc persistence across a DO restart", () => {
  let persistRoot: string;

  beforeEach(async () => {
    persistRoot = await mkdtemp(join(tmpdir(), "specular-do-"));
  });

  afterEach(async () => {
    await rm(persistRoot, { recursive: true, force: true });
  });

  it("survives a full miniflare restart via onSave/onLoad", async () => {
    const first = await bootServerHarness({ persistRoot });
    // The D1 rows (principal, doc, connection token) persist with the DO, so a
    // token minted against the first instance is still valid on the second.
    const principal = await signInAnonymous(first.url);
    const docId = await createDoc(first.url, { cookie: principal.cookie });
    const conn = await ownerConnect(first.url, { cookie: principal.cookie }, docId);

    const writer = new Y.Doc();
    const writerProvider = connectWithToken(first, docId, writer, conn.token);
    await waitFor(() => writerProvider.synced);
    writer.getMap("canvas").set("persisted", "on-disk");
    await delay(700); // onSave debounce flushes to the persisted DO storage.
    writerProvider.destroy();
    await first.dispose();

    // A brand-new instance over the same persist dir must reconstruct the doc
    // through onLoad — proving the chunked storage round-trips.
    const second = await bootServerHarness({ persistRoot });
    const reader = new Y.Doc();
    const readerProvider = connectWithToken(second, docId, reader, conn.token);
    await waitFor(() => reader.getMap("canvas").get("persisted") === "on-disk");
    expect(reader.getMap("canvas").get("persisted")).toBe("on-disk");
    readerProvider.destroy();
    await second.dispose();
  });
});
