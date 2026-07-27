import { apiKey } from "@better-auth/api-key";
import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { anonymous } from "better-auth/plugins";
import { drizzle } from "drizzle-orm/d1";

import type { Env } from "./env";
import * as schema from "./schema";

/**
 * Build the better-auth instance for a request, bound to the Worker's D1
 * database. Identity is account-shaped from day one (ADR 0018 §4): the
 * `anonymous` plugin mints the owner principal, `apiKey` issues account-scoped
 * agent credentials. Grants and capability links (step 3) hang off these rows.
 */
export function createAuth(env: Env) {
  const db = drizzle(env.DB, { schema });
  return betterAuth({
    database: drizzleAdapter(db, { provider: "sqlite", schema }),
    secret: env.BETTER_AUTH_SECRET,
    baseURL: env.BETTER_AUTH_URL,
    basePath: "/api/auth",
    plugins: [
      anonymous(),
      // Agents present their key via the `x-api-key` header and get resolved
      // to the owning principal's session — the agent-as-peer path (ADR 0018
      // §4, tier 3).
      apiKey({ enableSessionForAPIKeys: true }),
    ],
  });
}

export type Auth = ReturnType<typeof createAuth>;
