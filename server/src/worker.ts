import { routePartykitRequest, type Lobby } from "partyserver";

import { createAuth } from "./auth";
import { verifyConnectionToken } from "./connection-auth";
import type { Env } from "./env";
import { handleApiRequest } from "./routes";

export { CanvasDoc } from "./canvas-doc";

/**
 * Worker entry. Three surfaces:
 *  - `/api/auth/*` → better-auth (accounts, anonymous principals, api keys).
 *  - the cloud-sync HTTP surface (`/docs`, `/redeem`, `/assets`) → `routes.ts`.
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

    const api = await handleApiRequest(request, env);
    if (api) return api;

    const routed = await routePartykitRequest(request, env as never, {
      onBeforeConnect: async (req: Request, lobby: Lobby<Env>) => {
        // `lobby.name` is the DO name, i.e. the doc id from the URL path.
        const resolved = await verifyConnectionToken(req, env, lobby.name);
        if (!resolved) {
          return new Response("Unauthorized", { status: 401 });
        }
        // Stamp the resolved scope onto the upgrade request so the DO can
        // enforce read-only for view/comment connections without re-querying.
        const next = new Request(req);
        next.headers.set("x-specular-scope", resolved.scope);
        return next;
      },
    });
    if (routed) return routed;

    return new Response("Not found", { status: 404 });
  },
};
