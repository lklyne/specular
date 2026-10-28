/** Worker bindings, declared once and shared across the worker's modules. */
export interface Env {
  DB: D1Database;
  ASSETS: R2Bucket;
  /** The web viewer's static files (`web/`), served for every other path. */
  WEB?: Fetcher;
  /** One live room per shared canvas, named by its doc id. */
  LIVE_ROOM: DurableObjectNamespace;
  BETTER_AUTH_SECRET: string;
  BETTER_AUTH_URL: string;
}
