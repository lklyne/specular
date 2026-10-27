import { drizzle } from "drizzle-orm/d1";

import type { Env } from "./env";
import * as schema from "./schema";

export type Db = ReturnType<typeof getDb>;

/** One drizzle handle over the Worker's D1 binding, bound to the full schema. */
export function getDb(env: Env) {
  return drizzle(env.DB, { schema });
}

/** Capability scopes, widest-first. `edit` is the only writable scope. */
export type Scope = "view" | "comment" | "edit";

export const SCOPES: readonly Scope[] = ["view", "comment", "edit"];

export function isScope(value: unknown): value is Scope {
  return typeof value === "string" && (SCOPES as readonly string[]).includes(value);
}

/**
 * Opaque random token, hex-encoded. 32 bytes = 256 bits of entropy, comfortably
 * above the ≥128-bit floor the ADR requires for a link's security boundary.
 */
export function randomToken(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}
