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
      return withViewerHeaders(await env.WEB.fetch(new Request(page, request)), url);
    }

    return new Response("Not found", { status: 404 });
  },
};

/**
 * The viewer's own files run only its own script and WebAssembly, talk only
 * to this origin, and cannot be framed by another site.
 */
function withViewerHeaders(response: Response, url: URL): Response {
  const out = new Response(response.body, response);
  const socket = `${url.protocol === "https:" ? "wss:" : "ws:"}//${url.host}`;
  out.headers.set(
    "content-security-policy",
    [
      "default-src 'self'",
      "script-src 'self' 'wasm-unsafe-eval'",
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob:",
      `connect-src 'self' ${socket}`,
      "object-src 'none'",
      "base-uri 'none'",
      "frame-ancestors 'none'",
    ].join("; "),
  );
  out.headers.set("x-content-type-options", "nosniff");
  out.headers.set("referrer-policy", "no-referrer");
  return out;
}
