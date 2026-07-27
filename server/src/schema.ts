import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

/**
 * Drizzle schema for the better-auth tables. Table and column names match the
 * model/field names better-auth's core plus the `anonymous` and `apiKey`
 * plugins query by, verified against the field definitions in
 * `@better-auth/core` and `@better-auth/api-key` (see server/README.md).
 * The drizzle adapter resolves columns by these exact keys, so they must not
 * be renamed without regenerating the migration.
 */

export const user = sqliteTable("user", {
  id: text("id").primaryKey(),
  name: text("name").notNull(),
  email: text("email").notNull().unique(),
  emailVerified: integer("emailVerified", { mode: "boolean" })
    .notNull()
    .default(false),
  image: text("image"),
  createdAt: integer("createdAt", { mode: "timestamp" }).notNull(),
  updatedAt: integer("updatedAt", { mode: "timestamp" }).notNull(),
  // anonymous plugin
  isAnonymous: integer("isAnonymous", { mode: "boolean" }).default(false),
});

export const session = sqliteTable("session", {
  id: text("id").primaryKey(),
  expiresAt: integer("expiresAt", { mode: "timestamp" }).notNull(),
  token: text("token").notNull().unique(),
  createdAt: integer("createdAt", { mode: "timestamp" }).notNull(),
  updatedAt: integer("updatedAt", { mode: "timestamp" }).notNull(),
  ipAddress: text("ipAddress"),
  userAgent: text("userAgent"),
  userId: text("userId")
    .notNull()
    .references(() => user.id, { onDelete: "cascade" }),
});

export const account = sqliteTable("account", {
  id: text("id").primaryKey(),
  accountId: text("accountId").notNull(),
  providerId: text("providerId").notNull(),
  userId: text("userId")
    .notNull()
    .references(() => user.id, { onDelete: "cascade" }),
  accessToken: text("accessToken"),
  refreshToken: text("refreshToken"),
  idToken: text("idToken"),
  accessTokenExpiresAt: integer("accessTokenExpiresAt", { mode: "timestamp" }),
  refreshTokenExpiresAt: integer("refreshTokenExpiresAt", { mode: "timestamp" }),
  scope: text("scope"),
  password: text("password"),
  createdAt: integer("createdAt", { mode: "timestamp" }).notNull(),
  updatedAt: integer("updatedAt", { mode: "timestamp" }).notNull(),
});

export const verification = sqliteTable("verification", {
  id: text("id").primaryKey(),
  identifier: text("identifier").notNull(),
  value: text("value").notNull(),
  expiresAt: integer("expiresAt", { mode: "timestamp" }).notNull(),
  createdAt: integer("createdAt", { mode: "timestamp" }).notNull(),
  updatedAt: integer("updatedAt", { mode: "timestamp" }).notNull(),
});

/**
 * Cloud-sync ownership + capability-link tables (ADR 0018 §4). Every doc and
 * every grant hangs off an (anonymous) better-auth principal so the account
 * tier attaches to existing rows later with no migration.
 */

/** One row per synced canvas. `ownerId` is the principal that published it. */
export const docs = sqliteTable("docs", {
  docId: text("docId").primaryKey(),
  ownerId: text("ownerId")
    .notNull()
    .references(() => user.id, { onDelete: "cascade" }),
  createdAt: integer("createdAt", { mode: "timestamp" }).notNull(),
});

/**
 * One durable capability link per (docId, scope). `token` is an opaque random
 * lookup key (≥128-bit); revoke = delete the row, reset = rotate `token` in
 * place (same grantId). Never a self-contained signed token — validation is a
 * row lookup so links stay individually enumerable, revocable, and attributable.
 */
export const grants = sqliteTable("grants", {
  grantId: text("grantId").primaryKey(),
  docId: text("docId")
    .notNull()
    .references(() => docs.docId, { onDelete: "cascade" }),
  scope: text("scope").notNull(), // 'view' | 'comment' | 'edit'
  token: text("token").notNull().unique(),
  createdBy: text("createdBy").notNull(),
  createdAt: integer("createdAt", { mode: "timestamp" }).notNull(),
  expiresAt: integer("expiresAt", { mode: "timestamp" }),
});

/**
 * Short-TTL connection tokens minted by link redemption or the owner shortcut.
 * A redemption-derived token pins `parentGrantId` + `grantToken` (the parent's
 * token value at mint time); it stays valid only while the parent grant exists
 * with that same token generation, so a reset/revoke of the link transitively
 * kills every connection token derived from the old generation. Owner-shortcut
 * tokens have no parent (parentGrantId null) and are gated by ownership + TTL.
 */
export const connectionTokens = sqliteTable("connection_tokens", {
  token: text("token").primaryKey(),
  docId: text("docId").notNull(),
  scope: text("scope").notNull(),
  parentGrantId: text("parentGrantId"),
  grantToken: text("grantToken"),
  userId: text("userId"),
  expiresAt: integer("expiresAt", { mode: "timestamp" }).notNull(),
});

export const apikey = sqliteTable("apikey", {
  id: text("id").primaryKey(),
  configId: text("configId").notNull().default("default"),
  name: text("name"),
  start: text("start"),
  referenceId: text("referenceId").notNull(),
  prefix: text("prefix"),
  key: text("key").notNull(),
  refillInterval: integer("refillInterval"),
  refillAmount: integer("refillAmount"),
  lastRefillAt: integer("lastRefillAt", { mode: "timestamp" }),
  enabled: integer("enabled", { mode: "boolean" }).default(true),
  rateLimitEnabled: integer("rateLimitEnabled", { mode: "boolean" }).default(
    true,
  ),
  rateLimitTimeWindow: integer("rateLimitTimeWindow"),
  rateLimitMax: integer("rateLimitMax"),
  requestCount: integer("requestCount").default(0),
  remaining: integer("remaining"),
  lastRequest: integer("lastRequest", { mode: "timestamp" }),
  expiresAt: integer("expiresAt", { mode: "timestamp" }),
  createdAt: integer("createdAt", { mode: "timestamp" }).notNull(),
  updatedAt: integer("updatedAt", { mode: "timestamp" }).notNull(),
  permissions: text("permissions"),
  metadata: text("metadata"),
});
