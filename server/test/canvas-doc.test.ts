import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import WebSocket from "ws";
import YProvider from "y-partyserver/provider";
import * as Y from "yjs";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { bootServerHarness, type ServerHarness } from "./harness";

/** Provider host is scheme-less; localhost triggers the ws:// (not wss) path. */
function hostOf(url: string): string {
  return new URL(url).host;
}

function connect(url: string, docId: string, doc: Y.Doc): YProvider {
  return new YProvider(hostOf(url), docId, doc, {
    party: "canvas-doc",
    WebSocketPolyfill: WebSocket as unknown as typeof globalThis.WebSocket,
    // No BroadcastChannel: force sync through the server, not peer tabs.
    disableBc: true,
  });
}

async function waitFor(
  predicate: () => boolean,
  timeoutMs = 10_000,
): Promise<void> {
  const start = Date.now();
  while (!predicate()) {
    if (Date.now() - start > timeoutMs) throw new Error("waitFor timed out");
    await new Promise((r) => setTimeout(r, 20));
  }
}

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

describe("CanvasDoc sync", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  it("converges two headless Yjs clients through the DO", async () => {
    const docId = "converge-canvas";
    const docA = new Y.Doc();
    const docB = new Y.Doc();
    const providerA = connect(harness.url, docId, docA);
    const providerB = connect(harness.url, docId, docB);

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
    const docId = "persist-canvas";
    const writer = new Y.Doc();
    const writerProvider = connect(harness.url, docId, writer);
    await waitFor(() => writerProvider.synced);

    writer.getMap("canvas").set("kept", "value");
    // Let the DO's onSave debounce (200ms) flush to storage, then disconnect.
    await delay(700);
    writerProvider.destroy();
    await delay(200);

    const reader = new Y.Doc();
    const readerProvider = connect(harness.url, docId, reader);
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
    const docId = "durable-canvas";

    const first = await bootServerHarness({ persistRoot });
    const writer = new Y.Doc();
    const writerProvider = connect(first.url, docId, writer);
    await waitFor(() => writerProvider.synced);
    writer.getMap("canvas").set("persisted", "on-disk");
    await delay(700); // onSave debounce flushes to the persisted DO storage.
    writerProvider.destroy();
    await first.dispose();

    // A brand-new instance over the same persist dir must reconstruct the doc
    // through onLoad — proving the chunked storage round-trips.
    const second = await bootServerHarness({ persistRoot });
    const reader = new Y.Doc();
    const readerProvider = connect(second.url, docId, reader);
    await waitFor(() => reader.getMap("canvas").get("persisted") === "on-disk");
    expect(reader.getMap("canvas").get("persisted")).toBe("on-disk");
    readerProvider.destroy();
    await second.dispose();
  });
});
