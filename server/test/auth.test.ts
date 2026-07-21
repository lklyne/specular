import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { bootServerHarness, type ServerHarness } from "./harness";

let harness: ServerHarness;

beforeAll(async () => {
  harness = await bootServerHarness();
});

afterAll(async () => {
  await harness.dispose();
});

/** Read the D1 database directly to assert on persisted auth rows. */
async function queryFirst<T = Record<string, unknown>>(
  sql: string,
): Promise<T | null> {
  const db = await harness.mf.getD1Database("DB");
  const result = await db.prepare(sql).first<T>();
  return result ?? null;
}

describe("better-auth on D1", () => {
  it("anonymous sign-up creates a principal (user + session)", async () => {
    const res = await fetch(`${harness.url}/api/auth/sign-in/anonymous`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: "{}",
    });
    expect(res.status).toBe(200);

    const setCookie = res.headers.get("set-cookie");
    expect(setCookie).toContain("session_token");

    const body = (await res.json()) as { user?: { id: string } };
    expect(body.user?.id).toBeTruthy();

    const userRow = await queryFirst<{ id: string; isAnonymous: number }>(
      `SELECT id, isAnonymous FROM user WHERE id = '${body.user!.id}'`,
    );
    expect(userRow).not.toBeNull();
    expect(userRow!.isAnonymous).toBe(1);

    const sessionRow = await queryFirst<{ userId: string }>(
      `SELECT userId FROM session WHERE userId = '${body.user!.id}'`,
    );
    expect(sessionRow).not.toBeNull();
    expect(sessionRow!.userId).toBe(body.user!.id);
  });

  it("mints an api key and verifies it round-trip", async () => {
    // An api key is account-scoped, so mint it under an authenticated session.
    const signIn = await fetch(`${harness.url}/api/auth/sign-in/anonymous`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: "{}",
    });
    const owner = (await signIn.json()) as { user: { id: string } };
    const cookie = signIn.headers
      .getSetCookie()
      .map((c) => c.split(";")[0])
      .join("; ");

    const created = await fetch(`${harness.url}/api/auth/api-key/create`, {
      method: "POST",
      // Origin must be a trusted origin (baseURL) — better-auth's CSRF check
      // rejects cookie-authenticated writes with a missing/foreign Origin.
      headers: {
        "content-type": "application/json",
        origin: "http://localhost",
        cookie,
      },
      body: JSON.stringify({ name: "agent-token" }),
    });
    expect(created.status).toBe(200);
    const createdBody = (await created.json()) as { key?: string; id?: string };
    expect(createdBody.key).toBeTruthy();

    // The HTTP verify path is presenting the key: the plugin resolves an
    // `x-api-key` header into the owning principal's session.
    const session = await fetch(`${harness.url}/api/auth/get-session`, {
      headers: { "x-api-key": createdBody.key! },
    });
    expect(session.status).toBe(200);
    const sessionBody = (await session.json()) as {
      user?: { id: string };
    } | null;
    expect(sessionBody?.user?.id).toBe(owner.user.id);

    // Without the key, the same endpoint yields no session — the key is what
    // produced the principal above.
    const anon = await fetch(`${harness.url}/api/auth/get-session`);
    expect(await anon.json()).toBeNull();
  });
});
