// `specular-share`: publish a canvas to the Specular server and bring the
// comments made on it back. A stand-in for the desktop's own Share until
// that is built in Rust; it uses only the server's HTTP surface and, for
// page snapshots, the running app's local API.
//
//   node scripts/share.ts publish <file.canvas> [--server <url>] [--title <name>]
//   node scripts/share.ts pull <file.canvas>
//   node scripts/share.ts links <file.canvas>
//
// The owner's session and which doc each canvas was published as are kept
// in ~/.specular/share.json, never next to the canvas, so a shared folder
// never carries the credential.

import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";

const DEFAULT_SERVER = "https://specular-server.lyle-klyne.workers.dev";
const STATE_FILE = join(homedir(), ".specular", "share.json");
const DISCOVERY_FILES = ["specular-native-mcp.json", "specular-mcp.json"];

interface State {
  /** Server URL → the owner's session cookie there. */
  sessions: Record<string, string>;
  /** Absolute canvas path → where it was published, and as what. */
  canvases: Record<string, { server: string; docId: string; title?: string }>;
}

interface Manifest {
  format: 1;
  title: string;
  publishedAt: string;
  canvas: string;
  pages: Record<string, { asset: string; width: number; height: number }>;
  files: Record<string, string>;
}

type Json = Record<string, unknown>;

function fail(message: string): never {
  console.error(`specular-share: ${message}`);
  process.exit(1);
}

async function readState(): Promise<State> {
  try {
    return JSON.parse(await readFile(STATE_FILE, "utf8")) as State;
  } catch {
    return { sessions: {}, canvases: {} };
  }
}

async function writeState(state: State): Promise<void> {
  await mkdir(dirname(STATE_FILE), { recursive: true });
  await writeFile(STATE_FILE, JSON.stringify(state, null, 2), { mode: 0o600 });
}

async function call(
  url: string,
  init: RequestInit & { headers?: Record<string, string> } = {},
): Promise<Response> {
  const res = await fetch(url, init);
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    fail(`${init.method ?? "GET"} ${url} answered ${res.status} ${body}`);
  }
  return res;
}

/** The owner's session cookie on `server`, signing in anonymously once. */
async function session(state: State, server: string): Promise<string> {
  const known = state.sessions[server];
  if (known) return known;
  const res = await call(`${server}/api/auth/sign-in/anonymous`, {
    method: "POST",
    headers: { "content-type": "application/json", origin: server },
    body: "{}",
  });
  const cookie = res.headers
    .getSetCookie()
    .map((each) => each.split(";")[0])
    .join("; ");
  state.sessions[server] = cookie;
  await writeState(state);
  return cookie;
}

/** An edit connection token for `docId`, from the owner's session. */
async function editToken(server: string, cookie: string, docId: string): Promise<string> {
  const res = await call(`${server}/docs/${docId}/connect`, {
    method: "POST",
    headers: { cookie, origin: server },
  });
  return ((await res.json()) as { token: string }).token;
}

async function upload(
  server: string,
  docId: string,
  token: string,
  bytes: Uint8Array,
  contentType: string,
): Promise<string> {
  const hash = createHash("sha256").update(bytes).digest("hex");
  const head = await fetch(`${server}/assets/${hash}`, { method: "HEAD" });
  if (head.ok) return hash;
  const res = await call(`${server}/docs/${docId}/assets`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}`, "content-type": contentType },
    body: bytes.slice().buffer,
  });
  return ((await res.json()) as { assetId: string }).assetId;
}

function contentTypeOf(path: string): string {
  const extension = path.toLowerCase().split(".").pop() ?? "";
  const types: Record<string, string> = {
    png: "image/png",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    gif: "image/gif",
    webp: "image/webp",
    svg: "image/svg+xml",
    md: "text/markdown; charset=utf-8",
    markdown: "text/markdown; charset=utf-8",
  };
  return types[extension] ?? "application/octet-stream";
}

/** The files a canvas names that live on disk: images and Documents. */
function filesOf(canvas: Json): string[] {
  const nodes = Array.isArray(canvas.nodes) ? (canvas.nodes as Json[]) : [];
  const files = nodes
    .map((node) => node.file)
    .filter((file): file is string => typeof file === "string" && !/^[a-z]+:\/\//i.test(file));
  return [...new Set(files)];
}

/** The page entities of a canvas: JSON Canvas `link` nodes. */
function pagesOf(canvas: Json): string[] {
  const nodes = Array.isArray(canvas.nodes) ? (canvas.nodes as Json[]) : [];
  return nodes.filter((node) => node.type === "link").map((node) => String(node.id));
}

/** The running app's local API, if it is running. */
async function localApp(): Promise<{ url: string; secret: string } | null> {
  for (const name of DISCOVERY_FILES) {
    try {
      const found = JSON.parse(
        await readFile(join(homedir(), ".specular", name), "utf8"),
      ) as { port: number; secret: string };
      const url = `http://127.0.0.1:${found.port}`;
      const health = await fetch(`${url}/health`, {
        headers: { "x-specular-secret": found.secret },
      }).catch(() => null);
      if (health?.ok) return { url, secret: found.secret };
    } catch {
      // Not this one.
    }
  }
  return null;
}

/** A page's pixels as the open app draws them, or `null` when it cannot. */
async function snapshot(
  app: { url: string; secret: string },
  pageId: string,
): Promise<{ png: Uint8Array; width: number; height: number } | null> {
  const res = await fetch(`${app.url}/pages/screenshot`, {
    method: "POST",
    headers: { "content-type": "application/json", "x-specular-secret": app.secret },
    body: JSON.stringify({ pageId }),
  }).catch(() => null);
  if (!res?.ok) return null;
  const body = (await res.json()) as { base64?: string; width: number; height: number };
  if (!body.base64) return null;
  return { png: Buffer.from(body.base64, "base64"), width: body.width, height: body.height };
}

function option(args: string[], name: string): string | undefined {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

async function publish(path: string, args: string[]): Promise<void> {
  const state = await readState();
  const known = state.canvases[path];
  const server = (option(args, "--server") ?? known?.server ?? DEFAULT_SERVER).replace(/\/$/, "");
  const cookie = await session(state, server);

  let docId = known?.server === server ? known.docId : undefined;
  if (!docId) {
    const res = await call(`${server}/docs`, { method: "POST", headers: { cookie, origin: server } });
    docId = ((await res.json()) as { docId: string }).docId;
    state.canvases[path] = { server, docId };
    await writeState(state);
  }
  const token = await editToken(server, cookie, docId);
  const title =
    option(args, "--title") ?? known?.title ?? path.split("/").pop()!.replace(/\.canvas$/, "");
  if (title !== state.canvases[path].title) {
    state.canvases[path].title = title;
    await writeState(state);
  }

  const text = await readFile(path, "utf8");
  const canvas = JSON.parse(text) as Json;
  const folder = dirname(path);
  const manifest: Manifest = {
    format: 1,
    title,
    publishedAt: new Date().toISOString(),
    canvas: await upload(server, docId, token, new TextEncoder().encode(text), "application/json"),
    pages: {},
    files: {},
  };

  for (const file of filesOf(canvas)) {
    const bytes = await readFile(isAbsolute(file) ? file : join(folder, file)).catch(() => null);
    if (!bytes) {
      console.warn(`  skipped ${file}: not found next to the canvas`);
      continue;
    }
    manifest.files[file] = await upload(server, docId, token, bytes, contentTypeOf(file));
  }

  const pages = pagesOf(canvas);
  const app = pages.length ? await localApp() : null;
  if (pages.length && !app) {
    console.warn("  the app is not running: pages are published without snapshots");
  }
  for (const page of app ? pages : []) {
    const shot = await snapshot(app!, page);
    if (!shot) {
      console.warn(`  no snapshot of page ${page}: is this canvas the one open in the app?`);
      continue;
    }
    const asset = await upload(server, docId, token, shot.png, "image/png");
    manifest.pages[page] = { asset, width: shot.width, height: shot.height };
  }

  await call(`${server}/docs/${docId}/versions`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
    body: JSON.stringify(manifest),
  });
  console.log(
    `Published ${manifest.title}: ${Object.keys(manifest.pages).length} page snapshots, ` +
      `${Object.keys(manifest.files).length} files.`,
  );
  await printLinks(server, cookie, docId);
}

async function printLinks(server: string, cookie: string, docId: string): Promise<void> {
  for (const scope of ["comment", "view"]) {
    const res = await call(`${server}/docs/${docId}/links`, {
      method: "POST",
      headers: { cookie, origin: server, "content-type": "application/json" },
      body: JSON.stringify({ scope }),
    });
    const link = (await res.json()) as { url: string };
    console.log(`  ${scope === "comment" ? "Can comment" : "View only  "}  ${server}${link.url}`);
  }
}

/**
 * Merges the server's comments into the canvas file's `annotations`: the
 * server's copy of a comment replaces the file's, and the rest are added.
 * The app sees the file change and reloads it.
 */
async function pull(path: string): Promise<void> {
  const state = await readState();
  const known = state.canvases[path] ?? fail(`${path} has not been published`);
  const cookie = await session(state, known.server);
  const token = await editToken(known.server, cookie, known.docId);
  const res = await call(`${known.server}/docs/${known.docId}/comments`, {
    headers: { authorization: `Bearer ${token}` },
  });
  const { comments } = (await res.json()) as { comments: Json[] };

  const canvas = JSON.parse(await readFile(path, "utf8")) as Json;
  const annotations = Array.isArray(canvas.annotations) ? (canvas.annotations as Json[]) : [];
  const byId = new Map(comments.map((comment) => [comment.id, comment]));
  const merged = annotations.map((each) => byId.get(each.id) ?? each);
  const local = new Set(annotations.map((each) => each.id));
  const added = comments.filter((comment) => !local.has(comment.id));
  const updated = annotations.filter(
    (each) => byId.has(each.id) && JSON.stringify(byId.get(each.id)) !== JSON.stringify(each),
  );
  if (!added.length && !updated.length) {
    console.log("No new comments.");
    return;
  }
  canvas.annotations = [...merged, ...added];
  await writeFile(path, JSON.stringify(canvas, null, 2));
  console.log(`${added.length} new comments, ${updated.length} updated.`);
}

async function links(path: string): Promise<void> {
  const state = await readState();
  const known = state.canvases[path] ?? fail(`${path} has not been published`);
  await printLinks(known.server, await session(state, known.server), known.docId);
}

const [command, file, ...rest] = process.argv.slice(2);
if (!command || !file) {
  fail("usage: share.ts publish|pull|links <file.canvas> [--server <url>] [--title <name>]");
}
const path = resolve(file);
const commands: Record<string, () => Promise<void>> = {
  publish: () => publish(path, rest),
  pull: () => pull(path),
  links: () => links(path),
};
await (commands[command] ?? (() => fail(`unknown command ${command}`)))();
