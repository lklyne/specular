import type { CanvasDoc } from "./canvas-doc";

/** Worker bindings, declared once and shared across the worker and its Durable Object. */
export interface Env {
  CANVAS_DOC: DurableObjectNamespace<CanvasDoc>;
  DB: D1Database;
  ASSETS: R2Bucket;
  BETTER_AUTH_SECRET: string;
  BETTER_AUTH_URL: string;
}
