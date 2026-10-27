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

/**
 * Whether a connection token is refused for `docId`: the comments list is
 * the cheapest request every scope may make.
 */
export async function accessRejected(
  harness: ServerHarness,
  docId: string,
  token?: string,
): Promise<boolean> {
  const headers: Record<string, string> = {};
  if (token) headers.authorization = `Bearer ${token}`;
  const res = await fetch(`${harness.url}/docs/${docId}/comments`, { headers });
  return res.status === 401;
}

export async function uploadAsset(
  url: string,
  docId: string,
  token: string,
  bytes: Uint8Array,
  contentType?: string,
): Promise<Response> {
  const headers: Record<string, string> = { authorization: `Bearer ${token}` };
  if (contentType) headers["content-type"] = contentType;
  return fetch(`${url}/docs/${docId}/assets`, {
    method: "POST",
    headers,
    body: bytes.slice().buffer,
  });
}

/** Upload `text` and return its asset hash. */
export async function uploadText(
  url: string,
  docId: string,
  token: string,
  text: string,
): Promise<string> {
  const res = await uploadAsset(url, docId, token, new TextEncoder().encode(text));
  return ((await res.json()) as { assetId: string }).assetId;
}

/** A signed-in owner, a doc published once with a two-page canvas, and links. */
export async function publishedDoc(url: string): Promise<{
  principal: Principal;
  docId: string;
  editToken: string;
  comment: Link;
  view: Link;
}> {
  const { principal, docId, editToken } = await ownedEditDoc(url);
  const canvas = await uploadText(url, docId, editToken, CANVAS);
  const res = await fetch(`${url}/docs/${docId}/versions`, {
    method: "POST",
    headers: { authorization: `Bearer ${editToken}`, "content-type": "application/json" },
    body: JSON.stringify(manifest(canvas)),
  });
  if (res.status !== 201) throw new Error(`publish failed: ${res.status}`);
  const comment = await createLink(url, principal.cookie, docId, "comment");
  const view = await createLink(url, principal.cookie, docId, "view");
  return { principal, docId, editToken, comment, view };
}

export const CANVAS = JSON.stringify({
  nodes: [{ id: "p1", type: "link", x: 0, y: 0, width: 400, height: 300, url: "https://example.com" }],
  edges: [],
});

export function manifest(canvas: string, extra: Record<string, unknown> = {}) {
  return {
    format: 1,
    title: "Checkout",
    publishedAt: "2026-10-27T00:00:00.000Z",
    canvas,
    pages: {},
    files: {},
    ...extra,
  };
}

/** A redeemed link's connection token. */
export async function connect(url: string, link: Link): Promise<string> {
  const { body } = await redeem(url, link.token);
  if (!body.token) throw new Error("the link did not redeem");
  return body.token;
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
