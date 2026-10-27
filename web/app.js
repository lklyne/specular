// The web viewer's page. A share link opens /c/<docId>#t=<link token>: the
// token is redeemed for a short-lived connection token, the newest published
// version of the canvas is loaded into the app's own renderer
// (native/crates/specular-web, built into ./pkg), and comments go to and come
// from the server. Every route used here is in server/src/routes.ts.

import init, { WebViewer } from "./pkg/specular_web.js";

const POLL_MS = 5000;
const NAME_KEY = "specular.name";

const $ = (id) => document.getElementById(id);
const canvas = $("canvas");
const status = $("status");

const state = {
  docId: null,
  token: null,
  canComment: false,
  viewer: null,
  /** The server's comments, oldest first. */
  comments: [],
  /** Comments made here that are on their way to the server. */
  sending: [],
  showResolved: false,
  focused: null,
};

function show(message, isError = false) {
  status.textContent = message;
  status.classList.toggle("error", isError);
}

function storedName() {
  try {
    return localStorage.getItem(NAME_KEY);
  } catch {
    return null;
  }
}

function storeName(name) {
  try {
    localStorage.setItem(NAME_KEY, name);
  } catch {
    // A private window keeps nothing; the name is asked again next time.
  }
}

async function api(path, { method = "GET", body } = {}) {
  const headers = { authorization: `Bearer ${state.token}` };
  if (body !== undefined) headers["content-type"] = "application/json";
  const res = await fetch(path, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const json = await res.json().catch(() => ({}));
  if (!res.ok) throw Object.assign(new Error(json.error || res.statusText), { status: res.status });
  return json;
}

async function asset(hash) {
  const res = await fetch(`/assets/${hash}`);
  if (!res.ok) throw new Error(`asset ${hash} is missing`);
  return new Uint8Array(await res.arrayBuffer());
}

/** The link's doc id and token, from /c/<docId>#t=<token>. */
function linkOf(location) {
  const match = location.pathname.match(/^\/c\/([^/]+)\/?$/);
  const token = new URLSearchParams(location.hash.slice(1)).get("t");
  return match && token ? { docId: decodeURIComponent(match[1]), token } : null;
}

async function main() {
  const link = linkOf(window.location);
  if (!link) {
    show("This link is incomplete. Ask whoever shared it for a new one.", true);
    return;
  }
  state.docId = link.docId;
  show("Opening…");
  const redeemed = await fetch("/redeem", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ token: link.token }),
  });
  if (!redeemed.ok) {
    show("This link no longer works. Ask whoever shared it for a new one.", true);
    return;
  }
  const connection = await redeemed.json();
  state.token = connection.token;

  const { manifest, scope } = await api(`/docs/${state.docId}/manifest`).catch((error) => {
    throw new Error(error.status === 404 ? "Nothing has been published here yet." : error.message);
  });
  state.canComment = scope === "comment" || scope === "edit";
  $("title").textContent = manifest.title || "Untitled canvas";
  document.title = `${manifest.title || "Canvas"} · Specular`;
  $("mode-comment").hidden = !state.canComment;

  const [, canvasText] = await Promise.all([
    init(),
    asset(manifest.canvas).then((bytes) => new TextDecoder().decode(bytes)),
  ]);
  sizeCanvas();
  const seed = crypto.getRandomValues(new Uint32Array(2));
  state.viewer = await WebViewer.create(
    canvas,
    canvasText,
    state.canComment,
    seed[0] * 2 ** 21 + (seed[1] >>> 11),
  );
  resize();
  show("");

  loadFiles(manifest);
  wireInput();
  wireSidebar();
  await refreshComments();
  setInterval(() => document.visibilityState === "visible" && refreshComments(), POLL_MS);
  requestAnimationFrame(frame);
}

/** Pages' snapshots and the canvas's files, as they arrive. */
function loadFiles(manifest) {
  const viewer = state.viewer;
  const pages = Object.entries(manifest.pages || {}).map(([page, snapshot]) =>
    asset(snapshot.asset)
      .then((png) => viewer.add_snapshot(page, png))
      .catch((error) => console.warn(`page ${page}:`, error)),
  );
  const files = Object.entries(manifest.files || {}).map(([path, hash]) =>
    asset(hash)
      .then((bytes) => viewer.add_file(path, bytes))
      .catch((error) => console.warn(`file ${path}:`, error)),
  );
  Promise.all([...pages, ...files]).then(() => viewer.files_complete());
}

function sizeCanvas() {
  const rect = canvas.getBoundingClientRect();
  const scale = window.devicePixelRatio || 1;
  canvas.width = Math.max(1, Math.round(rect.width * scale));
  canvas.height = Math.max(1, Math.round(rect.height * scale));
  return { rect, scale };
}

function resize() {
  const { rect, scale } = sizeCanvas();
  state.viewer?.resize(rect.width, rect.height, scale);
}

function frame() {
  const viewer = state.viewer;
  try {
    viewer.frame(Date.now());
    canvas.style.cursor = viewer.cursor();
    sendNewComments();
    const focused = viewer.focused_comment() ?? null;
    if (focused !== state.focused) {
      state.focused = focused;
      renderThreads();
      document.querySelector(".thread.focused")?.scrollIntoView({ block: "nearest" });
    }
  } catch (error) {
    console.error(error);
  }
  requestAnimationFrame(frame);
}

// ---- Input -----------------------------------------------------------------

function modifiers(event) {
  return (event.shiftKey ? 1 : 0) | (event.ctrlKey ? 2 : 0) | (event.altKey ? 4 : 0) | (event.metaKey ? 8 : 0);
}

function local(event) {
  const rect = canvas.getBoundingClientRect();
  return [event.clientX - rect.left, event.clientY - rect.top];
}

function editingText(event) {
  const target = event.target;
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
}

function wireInput() {
  const viewer = state.viewer;
  canvas.addEventListener("pointerdown", (event) => {
    canvas.focus();
    canvas.setPointerCapture(event.pointerId);
    viewer.pointer("down", ...local(event), event.button, modifiers(event));
    event.preventDefault();
  });
  canvas.addEventListener("pointermove", (event) => {
    viewer.pointer("move", ...local(event), event.button, modifiers(event));
  });
  canvas.addEventListener("pointerup", (event) => {
    viewer.pointer("up", ...local(event), event.button, modifiers(event));
  });
  canvas.addEventListener("pointercancel", (event) => {
    viewer.pointer("up", ...local(event), event.button, modifiers(event));
  });
  canvas.addEventListener("pointerleave", (event) => {
    if (!canvas.hasPointerCapture(event.pointerId)) {
      viewer.pointer("leave", ...local(event), 0, modifiers(event));
    }
  });
  canvas.addEventListener("contextmenu", (event) => event.preventDefault());
  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      const unit = event.deltaMode === 1 ? 40 : event.deltaMode === 2 ? canvas.clientHeight : 1;
      viewer.pointer("move", ...local(event), 0, modifiers(event));
      viewer.wheel(event.deltaX * unit, event.deltaY * unit, modifiers(event));
    },
    { passive: false },
  );
  const key = (pressed) => (event) => {
    if (editingText(event) || document.querySelector("dialog[open]")) return;
    if (viewer.key(event.code, event.key, pressed, event.repeat, modifiers(event))) {
      event.preventDefault();
      return;
    }
    if (!pressed || event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "c" || event.key === "C") setMode("comment");
    else if (event.key === "v" || event.key === "V" || event.key === "h") setMode("pan");
  };
  window.addEventListener("keydown", key(true));
  window.addEventListener("keyup", key(false));
  new ResizeObserver(resize).observe(canvas);

  $("mode-pan").addEventListener("click", () => setMode("pan"));
  $("mode-comment").addEventListener("click", () => setMode("comment"));
  $("fit").addEventListener("click", () => viewer.zoom_to_fit());
}

async function setMode(mode) {
  if (mode === "comment") {
    if (!state.canComment) return;
    if (storedName() === null) await askName();
  }
  const now = state.viewer.set_mode(mode) ? mode : "pan";
  $("mode-pan").setAttribute("aria-pressed", String(now === "pan"));
  $("mode-comment").setAttribute("aria-pressed", String(now === "comment"));
  $("hint").hidden = now !== "comment";
  canvas.focus();
}

function askName() {
  const dialog = $("name-dialog");
  const input = $("name-input");
  input.value = storedName() || "";
  return new Promise((resolve) => {
    dialog.addEventListener(
      "close",
      () => {
        const name = dialog.returnValue === "ok" ? input.value.trim() : "";
        storeName(name);
        resolve(name);
      },
      { once: true },
    );
    dialog.showModal();
  });
}

// ---- Comments --------------------------------------------------------------

async function refreshComments() {
  try {
    const { comments } = await api(`/docs/${state.docId}/comments`);
    state.comments = comments;
    state.viewer.set_comments(JSON.stringify(comments));
    renderThreads();
  } catch (error) {
    console.warn("comments:", error);
  }
}

function sendNewComments() {
  const fresh = JSON.parse(state.viewer.take_new_comments());
  for (const annotation of fresh) {
    state.sending.push(annotation);
    renderThreads();
    api(`/docs/${state.docId}/comments`, {
      method: "POST",
      body: { annotation, authorName: storedName() || undefined },
    })
      .catch((error) => show(`Your comment was not saved: ${error.message}`, true))
      .finally(() => {
        state.sending = state.sending.filter((each) => each.id !== annotation.id);
        refreshComments();
      });
  }
}

function wireSidebar() {
  const sidebar = $("sidebar");
  const toggle = $("toggle-comments");
  toggle.addEventListener("click", () => {
    sidebar.hidden = !sidebar.hidden;
    toggle.setAttribute("aria-expanded", String(!sidebar.hidden));
    resize();
  });
  $("show-resolved").addEventListener("change", (event) => {
    state.showResolved = event.target.checked;
    renderThreads();
  });
}

const done = (comment) => comment.status === "resolved" || comment.status === "dismissed";

function when(iso) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const minutes = Math.round((Date.now() - date.getTime()) / 60000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  if (minutes < 60 * 24) return `${Math.round(minutes / 60)}h ago`;
  return date.toLocaleDateString();
}

function who(entry) {
  if (entry.authorName) return entry.authorName;
  if (entry.author === "agent") return "Agent";
  return entry.via === "web" ? "Guest" : "Owner";
}

function element(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  node.append(...children.filter((child) => child !== null && child !== undefined));
  return node;
}

function renderThreads() {
  // What the canvas shows: the comments it was published with, the server's,
  // and the ones on their way there.
  const all = JSON.parse(state.viewer.comments());
  const listed = new Set(state.comments.map((comment) => comment.id));
  const sending = new Set(state.sending.map((comment) => comment.id));
  for (const comment of all) {
    comment.sending = sending.has(comment.id) && !listed.has(comment.id);
    comment.onServer = listed.has(comment.id);
  }
  const open = all.filter((comment) => !done(comment)).length;
  $("count").textContent = open ? `(${open})` : "";
  const shown = all.filter((comment) => state.showResolved || !done(comment));
  shown.sort((a, b) => String(b.createdAt).localeCompare(String(a.createdAt)));
  $("empty").hidden = shown.length > 0;
  $("threads").replaceChildren(...shown.map(thread));
}

function thread(comment) {
  const item = element("li", {
    className: `thread${comment.id === state.focused ? " focused" : ""}${done(comment) ? " resolved" : ""}`,
  });
  item.addEventListener("click", (event) => {
    if (event.target.closest(".actions")) return;
    state.viewer.reveal_comment(comment.id);
  });
  item.append(
    element(
      "div",
      { className: "meta" },
      element("span", { className: "who", textContent: comment.sending ? "You" : who(comment) }),
      element("span", { textContent: comment.sending ? "sending…" : when(comment.createdAt) }),
      done(comment) ? element("span", { className: "state", textContent: "Resolved" }) : null,
    ),
    element("p", { className: "text", textContent: comment.text }),
  );
  if (comment.replies?.length) {
    item.append(
      element(
        "ul",
        { className: "replies" },
        ...comment.replies.map((reply) =>
          element(
            "li",
            {},
            element(
              "div",
              { className: "meta" },
              element("span", { className: "who", textContent: who(reply) }),
              element("span", { textContent: when(reply.timestamp) }),
            ),
            element("p", { className: "text", textContent: reply.text }),
          ),
        ),
      ),
    );
  }
  // Replies and resolving go through the server, which only holds the
  // comments made on the web; the desktop's own are answered there.
  if (state.canComment && comment.onServer) item.append(actions(comment));
  return item;
}

function actions(comment) {
  const input = element("input", { placeholder: "Reply…", maxLength: 10000 });
  const reply = element("button", { type: "submit", textContent: "Reply" });
  const resolve = element("button", {
    type: "button",
    textContent: done(comment) ? "Reopen" : "Resolve",
  });
  const form = element("form", { className: "actions" }, input, reply, resolve);
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const text = input.value.trim();
    if (!text) return;
    if (storedName() === null) await askName();
    input.disabled = true;
    await api(`/docs/${state.docId}/comments/${encodeURIComponent(comment.id)}/replies`, {
      method: "POST",
      body: { text, authorName: storedName() || undefined },
    }).catch((error) => show(`Your reply was not saved: ${error.message}`, true));
    refreshComments();
  });
  resolve.addEventListener("click", async () => {
    await api(`/docs/${state.docId}/comments/${encodeURIComponent(comment.id)}`, {
      method: "PATCH",
      body: { status: done(comment) ? "pending" : "resolved" },
    }).catch((error) => show(error.message, true));
    refreshComments();
  });
  return form;
}

main().catch((error) => {
  console.error(error);
  show(error.message || String(error), true);
});
