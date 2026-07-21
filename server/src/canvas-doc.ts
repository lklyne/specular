import { YServer } from "y-partyserver";
import * as Y from "yjs";

/**
 * DO storage caps a single value at 128 KiB, so the encoded doc update is
 * split across `ydoc:0..n-1` with a `ydoc:count` header. Writes go through one
 * `storage.put(object)` call, which is atomic, so a reader never sees a torn
 * mix of old and new chunks.
 */
const CHUNK_KEY_PREFIX = "ydoc:";
const CHUNK_COUNT_KEY = "ydoc:count";
const CHUNK_SIZE = 100_000; // < 128 KiB, leaving headroom for value encoding.

/** Save debounce, kept short so the spike's tests observe a flush quickly. */
const SAVE_DEBOUNCE_MS = 200;
const SAVE_DEBOUNCE_MAX_MS = 1_000;

function chunkKey(index: number): string {
  return `${CHUNK_KEY_PREFIX}${index}`;
}

/**
 * One Durable Object per canvas (`ADR 0018 §1`), keyed by doc id. Extends
 * `YServer` for the Yjs sync/awareness plumbing; the persistence obligation —
 * loading and saving the doc to DO storage so a canvas survives with no
 * clients connected — is ours, wired through `onLoad`/`onSave`.
 */
export class CanvasDoc extends YServer {
  static options = { hibernate: true };
  static callbackOptions = {
    debounceWait: SAVE_DEBOUNCE_MS,
    debounceMaxWait: SAVE_DEBOUNCE_MAX_MS,
  };

  async onLoad(): Promise<Y.Doc | void> {
    const storage = this.ctx.storage;
    const count = (await storage.get<number>(CHUNK_COUNT_KEY)) ?? 0;
    if (count === 0) return;

    const keys = Array.from({ length: count }, (_, i) => chunkKey(i));
    const stored = await storage.get<Uint8Array>(keys);
    const chunks: Uint8Array[] = [];
    let total = 0;
    for (const key of keys) {
      const chunk = stored.get(key);
      if (!chunk) return; // Incomplete write; treat as empty rather than corrupt.
      chunks.push(chunk);
      total += chunk.byteLength;
    }

    const update = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
      update.set(chunk, offset);
      offset += chunk.byteLength;
    }

    const doc = new Y.Doc();
    Y.applyUpdate(doc, update);
    return doc;
  }

  async onSave(): Promise<void> {
    const storage = this.ctx.storage;
    const update = Y.encodeStateAsUpdate(this.document);

    const entries: Record<string, Uint8Array | number> = {};
    let count = 0;
    for (let offset = 0; offset < update.byteLength; offset += CHUNK_SIZE) {
      entries[chunkKey(count)] = update.slice(offset, offset + CHUNK_SIZE);
      count += 1;
    }
    entries[CHUNK_COUNT_KEY] = count;

    const previousCount = (await storage.get<number>(CHUNK_COUNT_KEY)) ?? 0;
    await storage.put(entries);

    if (previousCount > count) {
      const stale = Array.from({ length: previousCount - count }, (_, i) =>
        chunkKey(count + i),
      );
      await storage.delete(stale);
    }
  }
}
