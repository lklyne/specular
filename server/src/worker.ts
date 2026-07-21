import { routePartykitRequest } from "partyserver";

import { createAuth } from "./auth";
import { verifyConnectionToken } from "./connection-auth";
import type { Env } from "./env";

export { CanvasDoc } from "./canvas-doc";

/**
 * Worker entry. Two surfaces:
 *  - `/api/auth/*` → better-auth (accounts, anonymous principals, api keys).
 *  - `/parties/canvas-doc/:docId` → the `CanvasDoc` Durable Object, routed by
 *    partyserver. `CANVAS_DOC` kebab-cases to the `canvas-doc` party; the doc
 *    id is the DO name. Upgrade auth runs in `onBeforeConnect`.
 */
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname.startsWith("/api/auth")) {
      return createAuth(env).handler(request);
    }

    const routed = await routePartykitRequest(request, env as never, {
      onBeforeConnect: (req: Request) => {
        if (!verifyConnectionToken(req)) {
          return new Response("Unauthorized", { status: 401 });
        }
      },
    });
    if (routed) return routed;

    return new Response("Not found", { status: 404 });
  },
};
