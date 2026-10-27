import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { build } from "esbuild";
import { Miniflare } from "miniflare";

const here = dirname(fileURLToPath(import.meta.url));
const serverRoot = join(here, "..");

export interface ServerHarness {
  /** Base URL of the live miniflare instance (real ephemeral port). */
  url: string;
  mf: Miniflare;
  dispose: () => Promise<void>;
}

async function bundleWorker(): Promise<string> {
  const result = await build({
    entryPoints: [join(serverRoot, "src/worker.ts")],
    bundle: true,
    format: "esm",
    platform: "neutral",
    conditions: ["workerd", "worker", "browser"],
    mainFields: ["module", "main"],
    external: ["node:*", "cloudflare:*"],
    write: false,
  });
  return result.outputFiles[0].text;
}

/** Migration files applied in order; extend as new migrations land. */
const MIGRATION_FILES = [
  "0000_init.sql",
  "0001_wandering_kang.sql",
  "0002_view_and_comment.sql",
];

async function applyMigrations(mf: Miniflare): Promise<void> {
  const db = await mf.getD1Database("DB");
  // Gate on the last-migration table so a persisted DB reused across boots
  // is not re-migrated, while a fresh DB runs every file.
  const existing = await db
    .prepare(
      "SELECT name FROM sqlite_master WHERE type='table' AND name='comments'",
    )
    .first();
  if (existing) return;

  for (const file of MIGRATION_FILES) {
    const sql = await readFile(join(serverRoot, "migrations", file), "utf8");
    const statements = sql
      .split("--> statement-breakpoint")
      .map((s) => s.trim())
      .filter(Boolean);
    for (const statement of statements) {
      await db.prepare(statement).run();
    }
  }
}

export interface HarnessOptions {
  /**
   * Directory for miniflare to persist D1 / R2 state between boots.
   */
  persistRoot?: string;
}

/**
 * Boots the Worker under miniflare with the DB / ASSETS bindings, applies the
 * migrations to D1, and exposes a real port for `fetch`.
 */
export async function bootServerHarness(
  options: HarnessOptions = {},
): Promise<ServerHarness> {
  const script = await bundleWorker();

  const mf = new Miniflare({
    modules: [{ type: "ESModule", path: "worker.js", contents: script }],
    modulesRoot: "/",
    compatibilityDate: "2026-07-14",
    compatibilityFlags: ["nodejs_compat"],
    defaultPersistRoot: options.persistRoot,
    d1Databases: { DB: "specular-auth-test" },
    r2Buckets: ["ASSETS"],
    bindings: {
      BETTER_AUTH_SECRET: "test-secret-please-change",
      BETTER_AUTH_URL: "http://localhost",
    },
    port: 0,
  });

  const ready = await mf.ready;
  await applyMigrations(mf);

  return {
    url: ready.toString().replace(/\/$/, ""),
    mf,
    dispose: () => mf.dispose(),
  };
}
