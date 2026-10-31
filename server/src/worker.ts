import { createAuth } from "./auth";
import type { Env } from "./env";
import { handleApiRequest } from "./routes";

export { LiveRoom } from "./live-room";

/**
 * Worker entry. Three surfaces:
 *  - `/api/auth/*` → better-auth (accounts, anonymous principals, api keys).
 *  - the HTTP surface in `routes.ts`: docs and their share links, link
 *    redemption, assets, published versions, comments and the websocket
 *    into a canvas's live room (`live-room.ts`).
 *  - everything else → the web viewer's static files (`web/`), whose
 *    `/c/:docId` page is the one a share link opens.
 */
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname.startsWith("/api/auth")) {
      return createAuth(env).handler(request);
    }

    const api = await handleApiRequest(request, env);
    if (api) return api;

    if (env.WEB && request.method === "GET") {
      // A link's page is the viewer's one page; its own path picks the doc.
      const page = url.pathname.startsWith("/c/") ? new URL("/", url) : url;
      return env.WEB.fetch(new Request(page, request));
    }

    return new Response("Not found", { status: 404 });
  },
};
