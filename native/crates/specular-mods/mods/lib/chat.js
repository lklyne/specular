// Agent chat for any web mod page: two elements and a few helpers, shipped
// with the app and rewritten on every start. A page in a mod's folder loads
// it with a classic script (file pages cannot import modules):
//
//   <script src="../lib/chat.js"></script>
//   <specular-thread thread="thread_abc"></specular-thread>
//   <specular-composer thread="thread_abc"></specular-composer>
//
// <specular-thread thread> draws a thread live from `chat/thread/<id>`:
// the messages with markdown, pasted images and what the agent made, the
// words streaming in, what the agent is doing, an error and the queue. It
// fires `thread` with the thread's value (null when closed or not made yet)
// each time it changes, for a title or a Stop button of the page's own.
//
// <specular-composer thread placeholder> is the agent panel's composer:
// the field, pasted images, the context pill and folder from
// `chat/context`, the model, and Send. Return sends and Shift+Return opens
// a line. It sends with `chat.send {text, images, thread}`, which starts
// the thread under that id if it is not made yet; with no `thread` it
// sends to the thread the panel has open. It fires a cancelable `send`
// with `{text, images, thread}` first; a listener may change `thread`.
//
// SpecularChat.ready(fn) runs fn once `window.specular` is there;
// SpecularChat.followTheme() keeps the `dark` class on <html> with the
// app's theme; SpecularChat.markdown(text) and SpecularChat.escape(text)
// are the readers the elements use. Colours are CSS variables (--sc-bg,
// --sc-ink, --sc-muted, --sc-rule, --sc-mine, --sc-code, --sc-accent,
// --sc-error) a page may set.
(() => {
  if (window.SpecularChat) return;

  const escape = (text) => String(text ?? "").replace(/[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

  // Fenced and inline code, headings, lists, bold, italics, links and
  // paragraphs. Text is escaped first, so nothing the agent writes runs.
  function inline(text) {
    return escape(text)
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
      .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>")
      .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a href="$2" target="_blank">$1</a>');
  }
  function markdown(source) {
    const out = [];
    const lines = String(source || "").split("\n");
    let list = null;
    let para = [];
    const flush = () => {
      if (para.length) out.push(`<p>${inline(para.join(" "))}</p>`);
      para = [];
    };
    const close = () => {
      if (list) out.push(`</${list}>`);
      list = null;
    };
    for (let n = 0; n < lines.length; n += 1) {
      const line = lines[n];
      if (/^```/.test(line)) {
        flush(); close();
        const code = [];
        for (n += 1; n < lines.length && !/^```/.test(lines[n]); n += 1) code.push(lines[n]);
        out.push(`<pre><code>${escape(code.join("\n"))}</code></pre>`);
        continue;
      }
      const heading = /^(#{1,6})\s+(.*)$/.exec(line);
      const bullet = /^\s*[-*]\s+(.*)$/.exec(line);
      const numbered = /^\s*\d+[.)]\s+(.*)$/.exec(line);
      if (heading) {
        flush(); close();
        const level = Math.min(heading[1].length, 4);
        out.push(`<h${level}>${inline(heading[2])}</h${level}>`);
      } else if (bullet || numbered) {
        flush();
        const kind = bullet ? "ul" : "ol";
        if (list !== kind) { close(); out.push(`<${kind}>`); list = kind; }
        out.push(`<li>${inline((bullet || numbered)[1])}</li>`);
      } else if (!line.trim()) {
        flush(); close();
      } else {
        close();
        para.push(line.trim());
      }
    }
    flush(); close();
    return out.join("");
  }

  function ready(fn) {
    if (window.specular) fn(window.specular);
    else addEventListener("specular-ready", () => fn(window.specular), { once: true });
  }

  function followTheme() {
    ready((specular) => specular.subscribe("theme", (theme) => {
      document.documentElement.classList.toggle("dark", Boolean(theme && theme.appearance === "dark"));
    }));
  }

  const STYLE = `
  :root { --sc-bg: #ffffff; --sc-ink: #1f2328; --sc-muted: #6e7781; --sc-rule: #d0d7de;
    --sc-mine: #f0f3f6; --sc-code: #f6f8fa; --sc-accent: #0969da; --sc-error: #cf222e; }
  :root.dark { --sc-bg: #1c1d1f; --sc-ink: #e6edf3; --sc-muted: #8b949e; --sc-rule: #30363d;
    --sc-mine: #2a2c30; --sc-code: #24262a; --sc-accent: #58a6ff; --sc-error: #ff7b72; }
  specular-thread { display: flex; flex-direction: column; gap: 10px; overflow-y: auto;
    padding: 10px; color: var(--sc-ink); min-height: 0; }
  specular-thread .msg { max-width: 100%; overflow-wrap: anywhere; }
  specular-thread .msg.user { align-self: flex-end; background: var(--sc-mine); padding: 6px 10px;
    border-radius: 10px; margin-left: 32px; white-space: pre-wrap; }
  specular-thread .msg.agent > :first-child { margin-top: 0; }
  specular-thread .msg.agent > :last-child { margin-bottom: 0; }
  specular-thread .msg p, specular-thread .msg ul, specular-thread .msg ol,
  specular-thread .msg pre { margin: 0 0 8px; }
  specular-thread .msg h1, specular-thread .msg h2, specular-thread .msg h3,
  specular-thread .msg h4 { margin: 10px 0 6px; font-size: 13px; }
  specular-thread .msg ul, specular-thread .msg ol { padding-left: 20px; }
  specular-thread code { font: 12px ui-monospace, "SF Mono", Menlo, monospace;
    background: var(--sc-code); padding: 1px 4px; border-radius: 4px; }
  specular-thread pre { background: var(--sc-code); padding: 8px; border-radius: 8px; overflow-x: auto; }
  specular-thread pre code { padding: 0; background: none; }
  specular-thread a { color: var(--sc-accent); }
  specular-thread .msg img { max-width: 100%; border-radius: 6px; display: block; margin-top: 6px; }
  specular-thread .card { font-size: 12px; color: var(--sc-muted); border: 1px solid var(--sc-rule);
    border-radius: 6px; padding: 3px 8px; margin-top: 4px; }
  specular-thread .note { color: var(--sc-muted); font-size: 12px; }
  specular-thread .error { color: var(--sc-error); font-size: 12px; }
  specular-thread .queued { font-size: 12px; color: var(--sc-muted); border: 1px dashed var(--sc-rule);
    border-radius: 6px; padding: 3px 8px; align-self: flex-end; }
  specular-thread .empty { margin: auto; text-align: center; color: var(--sc-muted); padding: 16px; }
  specular-composer { display: flex; flex-direction: column; gap: 6px; padding: 8px;
    border: 1px solid var(--sc-rule); border-radius: 14px; background: var(--sc-bg); color: var(--sc-ink); }
  specular-composer:focus-within { border-color: var(--sc-accent); }
  specular-composer textarea { width: 100%; resize: none; border: 0; outline: none; padding: 2px;
    font: inherit; color: inherit; background: none; min-height: 40px; max-height: 140px; }
  specular-composer .row { display: flex; align-items: center; gap: 4px; min-width: 0; }
  specular-composer .chip { font-size: 11px; color: var(--sc-muted); border-radius: 999px; padding: 1px 6px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 45%; }
  specular-composer .chip:empty { display: none; }
  specular-composer .spacer { flex: 1; }
  specular-composer select { font: inherit; font-size: 11px; color: var(--sc-muted); background: none;
    border: 0; max-width: 120px; cursor: pointer; }
  specular-composer .send { width: 26px; height: 26px; border-radius: 13px; border: 0; cursor: pointer;
    background: var(--sc-accent); color: #fff; font-size: 13px; }
  specular-composer .send:disabled { opacity: 0.35; cursor: default; }
  specular-composer .thumbs { display: flex; flex-wrap: wrap; gap: 6px; }
  specular-composer .thumbs:empty { display: none; }
  specular-composer .thumb { position: relative; width: 48px; height: 48px; }
  specular-composer .thumb img { width: 100%; height: 100%; object-fit: cover; border-radius: 6px;
    border: 1px solid var(--sc-rule); display: block; }
  specular-composer .thumb button { position: absolute; top: 2px; right: 2px; width: 16px; height: 16px;
    padding: 0; border: 0; border-radius: 50%; background: #000b; color: #fff; font-size: 11px;
    line-height: 16px; cursor: pointer; }`;
  const sheet = document.createElement("style");
  sheet.textContent = STYLE;
  document.head.prepend(sheet);

  const fileUrl = (path) => (/^[a-z]+:/i.test(path) ? path : `file://${encodeURI(path)}`);
  function message(m) {
    const images = (m.images || []).map((path) => `<img src="${escape(fileUrl(path))}">`).join("");
    if (m.role === "user") return `<div class="msg user">${escape(m.text)}${images}</div>`;
    const cards = (m.artifacts || []).map((card) =>
      `<div class="card">${card.created ? "Made" : "Edited"} ${escape(card.label)} · ${escape(card.detail)}</div>`);
    return `<div class="msg agent">${markdown(m.text)}${images}${cards.join("")}</div>`;
  }

  class SpecularThread extends HTMLElement {
    static get observedAttributes() { return ["thread"]; }
    connectedCallback() { this.follow(); }
    disconnectedCallback() { this.stop(); }
    attributeChangedCallback() { if (this.isConnected) this.follow(); }
    stop() {
      if (this.off) this.off();
      this.off = null;
    }
    follow() {
      const id = this.getAttribute("thread");
      if (id === this.following && this.off) return;
      this.stop();
      this.following = id;
      this.value = null;
      this.paint();
      if (!id) return;
      ready((specular) => {
        if (this.following !== id) return;
        this.off = specular.subscribe(`chat/thread/${id}`, (value) => {
          this.value = value;
          this.paint();
          this.dispatchEvent(new CustomEvent("thread", { detail: value }));
        });
      });
    }
    paint() {
      const t = this.value;
      const near = this.scrollHeight - this.scrollTop - this.clientHeight < 40;
      if (!t) {
        const hint = this.getAttribute("empty") || "Write below to start a thread.";
        this.innerHTML = `<div class="empty">${escape(hint)}</div>`;
        return;
      }
      const parts = t.messages.map(message);
      if (t.streaming) parts.push(message({ role: "agent", text: t.streaming, artifacts: t.making }));
      if (t.run) parts.push(`<div class="note">${escape(t.run.label)}</div>`);
      if (t.error) parts.push(`<div class="error">${escape(t.error)}</div>`);
      for (const text of t.queued || []) parts.push(`<div class="queued">Queued: ${escape(text)}</div>`);
      if (!parts.length) parts.push(`<div class="empty">${escape(t.emptyHint || "Write below to start.")}</div>`);
      this.innerHTML = parts.join("");
      if (near) this.scrollTop = this.scrollHeight;
    }
  }

  // The formats a thread takes; anything else pasted is left to the field.
  const TAKES = ["image/png", "image/jpeg", "image/gif", "image/webp"];
  const PILL = { dom: "Element", comment: "Comment", selection: "Selection", canvas: "Canvas" };

  class SpecularComposer extends HTMLElement {
    static get observedAttributes() { return ["placeholder"]; }
    connectedCallback() {
      if (this.field) return;
      this.images = [];
      this.innerHTML = `<div class="thumbs"></div><textarea rows="1"></textarea>
        <div class="row"><span class="chip pill"></span><span class="chip folder"></span>
        <span class="spacer"></span><select title="The model runs use" hidden></select>
        <button class="send" title="Send" disabled>↑</button></div>`;
      this.field = this.querySelector("textarea");
      this.field.placeholder = this.getAttribute("placeholder") || "Write to the agent…";
      this.field.addEventListener("keydown", (event) => {
        if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
          event.preventDefault();
          this.send();
        }
      });
      this.field.addEventListener("input", () => {
        this.field.style.height = "auto";
        this.field.style.height = `${this.field.scrollHeight}px`;
        this.ready();
      });
      this.field.addEventListener("paste", (event) => {
        const files = event.clipboardData ? event.clipboardData.files : [];
        if (this.take(files)) event.preventDefault();
      });
      this.querySelector(".send").addEventListener("click", () => {
        this.send();
        this.field.focus();
      });
      this.querySelector(".thumbs").addEventListener("click", (event) => {
        const n = event.target.dataset && event.target.dataset.n;
        if (n === undefined) return;
        this.images.splice(Number(n), 1);
        this.thumbs();
      });
      this.querySelector("select").addEventListener("change", (event) =>
        window.specular && window.specular.run("chat.model", { name: event.target.value }));
      ready((specular) => specular.subscribe("chat/context", (value) => this.context(value)));
    }
    attributeChangedCallback() {
      if (this.field) this.field.placeholder = this.getAttribute("placeholder") || "Write to the agent…";
    }
    focus() { if (this.field) this.field.focus(); }
    ready() {
      this.querySelector(".send").disabled = !(this.field.value.trim() || this.images.length);
    }
    thumbs() {
      this.querySelector(".thumbs").innerHTML = this.images
        .map((image, n) => `<div class="thumb"><img src="${image.data}"><button data-n="${n}" title="Remove">×</button></div>`)
        .join("");
      this.ready();
    }
    take(files) {
      const images = [...files].filter((file) => TAKES.includes(file.type));
      for (const file of images) {
        const reader = new FileReader();
        reader.onload = () => {
          this.images.push({ type: file.type, data: reader.result });
          this.thumbs();
        };
        reader.readAsDataURL(file);
      }
      return images.length > 0;
    }
    context(value) {
      if (!value) return;
      const pill = value.pill || {};
      this.querySelector(".pill").textContent = pill.label ? `${PILL[pill.kind] || "Canvas"} · ${pill.label}` : "";
      const folder = this.querySelector(".folder");
      folder.textContent = value.folder || "";
      folder.title = value.folderPath ? `Changes are written to ${value.folderPath}` : "";
      const options = (value.model && value.model.options) || [];
      const select = this.querySelector("select");
      select.innerHTML = options
        .map((o) => `<option value="${escape(o.value || o.name)}"${o.chosen ? " selected" : ""}>${escape(o.name)}</option>`)
        .join("");
      select.hidden = options.length === 0;
    }
    send() {
      const text = this.field.value.trim();
      const images = this.images;
      if (!text && !images.length) return;
      const thread = this.getAttribute("thread") || undefined;
      const detail = { text, images, thread };
      if (!this.dispatchEvent(new CustomEvent("send", { detail, cancelable: true }))) return;
      this.field.value = "";
      this.field.style.height = "auto";
      this.images = [];
      this.thumbs();
      ready((specular) => specular.run("chat.send", detail).catch((error) => console.error(error)));
    }
  }

  customElements.define("specular-thread", SpecularThread);
  customElements.define("specular-composer", SpecularComposer);
  window.SpecularChat = Object.freeze({ markdown, escape, ready, followTheme });
})();
