import { DurableObject } from "cloudflare:workers";
import * as Y from "yjs";

import type { Scope } from "./db";
import type { Env } from "./env";

/**
 * The live room of one shared canvas (ADR 0050): every edit peer's yrs
 * updates, kept and passed on to every other peer. The room never builds
 * the document; it only merges updates, so it holds no canvas logic.
 *
 * Frames are binary, a tag byte then the payload:
 *  - `STATE` (server → peer, once on connect): every update so far, merged.
 *    Empty when nobody has shared the canvas yet.
 *  - `UPDATE` (both ways): one yrs v1 update. Only edit peers may send one.
 */
export const STATE = 0;
export const UPDATE = 1;

/** Past this many stored updates the room merges them into one row. */
const COMPACT_AFTER = 64;

interface Peer {
  scope: Scope;
}

export class LiveRoom extends DurableObject<Env> {
  /** The stored updates, read from SQLite on the first use after a wake. */
  private updates: Uint8Array[] | null = null;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS updates (seq INTEGER PRIMARY KEY AUTOINCREMENT, data BLOB NOT NULL)",
    );
  }

  /** The upgrade, already authorised by the worker, which names the scope. */
  async fetch(request: Request): Promise<Response> {
    if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") {
      return new Response("expected a websocket", { status: 426 });
    }
    const scope = request.headers.get("x-specular-scope") as Scope | null;
    if (!scope) return new Response("no scope", { status: 400 });

    const { 0: client, 1: server } = new WebSocketPair();
    this.ctx.acceptWebSocket(server);
    server.serializeAttachment({ scope } satisfies Peer);
    server.send(frame(STATE, this.state()));
    return new Response(null, { status: 101, webSocket: client });
  }

  async webSocketMessage(ws: WebSocket, message: string | ArrayBuffer): Promise<void> {
    if (typeof message === "string") return;
    const bytes = new Uint8Array(message);
    const peer = ws.deserializeAttachment() as Peer;
    if (bytes[0] !== UPDATE) return;
    if (peer.scope !== "edit") {
      ws.close(4003, "this link cannot edit");
      return;
    }
    const update = bytes.subarray(1);
    try {
      // Refuses bytes that are not an update before anyone else gets them.
      Y.mergeUpdates([update]);
    } catch {
      ws.close(4000, "not an update");
      return;
    }
    this.keep(update);
    const out = frame(UPDATE, update);
    for (const other of this.ctx.getWebSockets()) {
      if (other !== ws) other.send(out);
    }
  }

  async webSocketClose(ws: WebSocket, code: number, reason: string): Promise<void> {
    try {
      ws.close(code, reason);
    } catch {
      // Already closed by the runtime's own reply.
    }
  }

  private stored(): Uint8Array[] {
    if (!this.updates) {
      this.updates = this.ctx.storage.sql
        .exec<{ data: ArrayBuffer }>("SELECT data FROM updates ORDER BY seq")
        .toArray()
        .map((row) => new Uint8Array(row.data));
    }
    return this.updates;
  }

  private state(): Uint8Array {
    const updates = this.stored();
    if (updates.length === 0) return new Uint8Array();
    return updates.length === 1 ? updates[0] : Y.mergeUpdates(updates);
  }

  private keep(update: Uint8Array): void {
    const updates = this.stored();
    updates.push(update.slice());
    this.ctx.storage.sql.exec("INSERT INTO updates (data) VALUES (?)", update);
    if (updates.length > COMPACT_AFTER) {
      const merged = Y.mergeUpdates(updates);
      this.ctx.storage.transactionSync(() => {
        this.ctx.storage.sql.exec("DELETE FROM updates");
        this.ctx.storage.sql.exec("INSERT INTO updates (data) VALUES (?)", merged);
      });
      this.updates = [merged];
    }
  }
}

function frame(tag: number, payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(payload.length + 1);
  out[0] = tag;
  out.set(payload, 1);
  return out;
}
