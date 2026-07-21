import WebSocket from "ws";
import YProvider from "y-partyserver/provider";
import * as Y from "yjs";

import type { ServerHarness } from "./harness";

export interface Principal {
  userId: string;
  /** `name=value` cookie header for authenticated calls. */
  cookie: string;
}

/** Sign in anonymously and return the principal id + session cookie. */
export async function signInAnonymous(url: string): Promise<Principal> {
  const res = await fetch(`${url}/api/auth/sign-in/anonymous`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: "{}",
  });
  const body = (await res.json()) as { user: { id: string } };
  const cookie = res.headers
    .getSetCookie()
    .map((c) => c.split(";")[0])
    .join("; ");
  return { userId: body.user.id, cookie };
}

/** Mint an api key under a principal (for the x-api-key owner-auth path). */
export async function createApiKey(
  url: string,
  cookie: string,
): Promise<string> {
  const res = await fetch(`${url}/api/auth/api-key/create`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      origin: "http://localhost",
      cookie,
    },
    body: JSON.stringify({ name: "agent" }),
  });
  const body = (await res.json()) as { key: string };
  return body.key;
}

export async function createDoc(
  url: string,
  auth: { cookie?: string; apiKey?: string },
): Promise<string> {
  const res = await fetch(`${url}/docs`, {
    method: "POST",
    headers: authHeaders(auth),
  });
  const body = (await res.json()) as { docId: string };
  return body.docId;
}

export interface Link {
  grantId: string;
  scope: string;
  token: string;
  url: string;
}

export async function createLink(
  url: string,
  cookie: string,
  docId: string,
  scope: string,
): Promise<Link> {
  const res = await fetch(`${url}/docs/${docId}/links`, {
    method: "POST",
    headers: { "content-type": "application/json", cookie },
    body: JSON.stringify({ scope }),
  });
  return (await res.json()) as Link;
}

export interface ConnectionToken {
  token: string;
  docId: string;
  scope: string;
  expiresAt: number;
}

export async function redeem(
  url: string,
  token: string,
): Promise<{ status: number; body: Partial<ConnectionToken> }> {
  const res = await fetch(`${url}/redeem`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ token }),
  });
  return { status: res.status, body: (await res.json()) as Partial<ConnectionToken> };
}

export async function ownerConnect(
  url: string,
  auth: { cookie?: string; apiKey?: string },
  docId: string,
): Promise<ConnectionToken> {
  const res = await fetch(`${url}/docs/${docId}/connect`, {
    method: "POST",
    headers: authHeaders(auth),
  });
  return (await res.json()) as ConnectionToken;
}

function authHeaders(auth: { cookie?: string; apiKey?: string }): HeadersInit {
  const headers: Record<string, string> = { "content-type": "application/json" };
  if (auth.cookie) headers.cookie = auth.cookie;
  if (auth.apiKey) headers["x-api-key"] = auth.apiKey;
  return headers;
}

/** Provider host is scheme-less; localhost triggers the ws:// path. */
function hostOf(url: string): string {
  return new URL(url).host;
}

/**
 * Connect a Yjs client through the DO, presenting a connection token as the
 * `?token=` query param the upgrade auth reads.
 */
export function connectWithToken(
  harness: ServerHarness,
  docId: string,
  doc: Y.Doc,
  token: string,
): YProvider {
  return new YProvider(hostOf(harness.url), docId, doc, {
    party: "canvas-doc",
    params: { token },
    WebSocketPolyfill: WebSocket as unknown as typeof globalThis.WebSocket,
    disableBc: true,
  });
}

export async function waitFor(
  predicate: () => boolean,
  timeoutMs = 10_000,
): Promise<void> {
  const start = Date.now();
  while (!predicate()) {
    if (Date.now() - start > timeoutMs) throw new Error("waitFor timed out");
    await new Promise((r) => setTimeout(r, 20));
  }
}

export const delay = (ms: number): Promise<void> =>
  new Promise((r) => setTimeout(r, ms));

/**
 * Open a raw WebSocket against the DO and resolve whether the upgrade was
 * rejected (no 101). Used to assert invalid/missing tokens can't even connect.
 */
export function upgradeRejected(
  harness: ServerHarness,
  docId: string,
  token?: string,
): Promise<boolean> {
  const wsBase = harness.url.replace(/^http/, "ws");
  const query = token ? `?token=${encodeURIComponent(token)}` : "";
  const ws = new WebSocket(`${wsBase}/parties/canvas-doc/${docId}${query}`);
  return new Promise<boolean>((resolve) => {
    ws.on("open", () => {
      ws.close();
      resolve(false); // upgrade succeeded → not rejected
    });
    ws.on("unexpected-response", () => resolve(true));
    ws.on("error", () => resolve(true));
  });
}

/** Convenience: sign in, create a doc, and mint an edit connection token. */
export async function ownedEditDoc(
  url: string,
): Promise<{ principal: Principal; docId: string; editToken: string }> {
  const principal = await signInAnonymous(url);
  const docId = await createDoc(url, { cookie: principal.cookie });
  const conn = await ownerConnect(url, { cookie: principal.cookie }, docId);
  return { principal, docId, editToken: conn.token };
}
