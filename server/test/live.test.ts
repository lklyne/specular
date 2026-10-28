import { afterEach, beforeEach, describe, expect, it } from "vitest";
import * as Y from "yjs";

import { bootServerHarness, type ServerHarness } from "./harness";
import { connect, createLink, ownedEditDoc } from "./helpers";

const STATE = 0;
const UPDATE = 1;

/** One peer's socket into a room, with every frame it has had. */
class Peer {
  frames: Uint8Array[] = [];
  closed: { code: number } | null = null;
  private waiters: (() => void)[] = [];

  private constructor(readonly socket: WebSocket) {
    socket.binaryType = "arraybuffer";
    socket.addEventListener("message", (event) => {
      this.frames.push(new Uint8Array(event.data as ArrayBuffer));
      this.wake();
    });
    socket.addEventListener("close", (event) => {
      this.closed = { code: event.code };
      this.wake();
    });
  }

  static async open(url: string, docId: string, token: string): Promise<Peer> {
    const socket = new WebSocket(`${url.replace(/^http/, "ws")}/docs/${docId}/live?token=${token}`);
    const peer = new Peer(socket);
    await new Promise<void>((resolve, reject) => {
      socket.addEventListener("open", () => resolve());
      socket.addEventListener("error", () => reject(new Error("socket refused")));
    });
    return peer;
  }

  /** Waits until `count` frames have come, or the socket closed. */
  async until(count: number): Promise<void> {
    while (this.frames.length < count && !this.closed) {
      await new Promise<void>((resolve) => {
        this.waiters.push(resolve);
        setTimeout(resolve, 5_000);
      });
    }
  }

  send(tag: number, payload: Uint8Array): void {
    const out = new Uint8Array(payload.length + 1);
    out[0] = tag;
    out.set(payload, 1);
    this.socket.send(out);
  }

  close(): void {
    this.socket.close();
  }

  private wake(): void {
    for (const waiter of this.waiters.splice(0)) waiter();
  }
}

/** A Y.Doc with `key` set to `value` in the `entities` map, as one update. */
function edit(doc: Y.Doc, key: string, value: string): Uint8Array {
  const before = Y.encodeStateVector(doc);
  doc.getMap("entities").set(key, value);
  return Y.encodeStateAsUpdate(doc, before);
}

function entities(update: Uint8Array): Record<string, unknown> {
  const doc = new Y.Doc();
  if (update.length) Y.applyUpdate(doc, update);
  return doc.getMap("entities").toJSON();
}

describe("a canvas's live room", () => {
  let harness: ServerHarness;

  beforeEach(async () => {
    harness = await bootServerHarness();
  });

  afterEach(async () => {
    await harness.dispose();
  });

  async function room() {
    const { principal, docId, editToken } = await ownedEditDoc(harness.url);
    const edit = await createLink(harness.url, principal.cookie, docId, "edit");
    const view = await createLink(harness.url, principal.cookie, docId, "view");
    return {
      docId,
      ownerToken: editToken,
      editToken: await connect(harness.url, edit),
      viewToken: await connect(harness.url, view),
    };
  }

  it("relays an edit peer's updates and gives a late peer everything so far", async () => {
    const { docId, ownerToken, editToken } = await room();
    const ann = await Peer.open(harness.url, docId, ownerToken);
    await ann.until(1);
    expect(ann.frames[0]).toEqual(new Uint8Array([STATE]));

    const annDoc = new Y.Doc();
    ann.send(UPDATE, edit(annDoc, "s1", "sticky"));
    const ben = await Peer.open(harness.url, docId, editToken);
    await ben.until(1);
    expect(ben.frames[0][0]).toBe(STATE);
    expect(entities(ben.frames[0].subarray(1))).toEqual({ s1: "sticky" });

    ann.send(UPDATE, edit(annDoc, "s2", "page"));
    await ben.until(2);
    expect(ben.frames[1][0]).toBe(UPDATE);
    const benDoc = new Y.Doc();
    Y.applyUpdate(benDoc, ben.frames[0].subarray(1));
    Y.applyUpdate(benDoc, ben.frames[1].subarray(1));
    expect(benDoc.getMap("entities").toJSON()).toEqual({ s1: "sticky", s2: "page" });
    // A peer does not get its own update back.
    expect(ann.frames).toHaveLength(1);
    ann.close();
    ben.close();
  });

  it("lets a view link listen but not edit", async () => {
    const { docId, ownerToken, viewToken } = await room();
    const viewer = await Peer.open(harness.url, docId, viewToken);
    await viewer.until(1);
    const owner = await Peer.open(harness.url, docId, ownerToken);
    await owner.until(1);
    owner.send(UPDATE, edit(new Y.Doc(), "s1", "sticky"));
    await viewer.until(2);
    expect(entities(viewer.frames[1].subarray(1))).toEqual({ s1: "sticky" });

    viewer.send(UPDATE, edit(new Y.Doc(), "s9", "vandal"));
    await viewer.until(99);
    expect(viewer.closed?.code).toBe(4003);
    const late = await Peer.open(harness.url, docId, ownerToken);
    await late.until(1);
    expect(entities(late.frames[0].subarray(1))).toEqual({ s1: "sticky" });
    owner.close();
    late.close();
  });

  it("refuses a socket without a token for this canvas", async () => {
    const { docId } = await room();
    const other = await room();
    await expect(Peer.open(harness.url, docId, "nope")).rejects.toThrow();
    await expect(Peer.open(harness.url, docId, other.ownerToken)).rejects.toThrow();
  });

  it("keeps every update when it merges them into one", async () => {
    const { docId, ownerToken } = await room();
    const owner = await Peer.open(harness.url, docId, ownerToken);
    await owner.until(1);
    const doc = new Y.Doc();
    const wanted: Record<string, string> = {};
    for (let i = 0; i < 100; i++) {
      owner.send(UPDATE, edit(doc, `s${i}`, `n${i}`));
      wanted[`s${i}`] = `n${i}`;
    }
    // A listener that joins after the last send has every update in order.
    const probe = await Peer.open(harness.url, docId, ownerToken);
    await probe.until(1);
    expect(entities(probe.frames[0].subarray(1))).toEqual(wanted);
    owner.close();
    probe.close();
  });
});
