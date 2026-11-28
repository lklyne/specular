// The agent panel for any web mod page, shipped with the app and rewritten
// on every start. A page in a mod's folder loads it with a classic script
// (a file:// page cannot import a module); it brings chat.css beside it:
//
//   <script src="../lib/chat.js"></script>
//   <specular-chat thread="thread_abc"></specular-chat>
//
// <specular-chat thread> is the whole right-panel thread: the header (the
// title, which switches thread, then New, Archive and Close), the
// transcript and the composer. It changes its own `thread` attribute as a
// person switches or starts one and fires `thread-change` with
// `{thread}`, so a page can keep it; Close fires `close`, and the button
// shows only with a `closable` attribute.
//
// The parts stand alone too. <specular-thread thread empty> draws a
// thread live from `chat/thread/<id>` and fires `thread` with its value
// (null when closed or not made yet) each time it changes.
// <specular-composer thread placeholder> is the panel's composer: queued
// words, pasted images, the field, the context pill and folder from
// `chat/context`, the model and Send. Return sends, Shift+Return opens a
// line. It runs `chat.send {text, images, thread}`, which starts the
// thread under that id if it is not made yet (no `thread`: the panel's
// open thread), after a cancelable `send` event whose listener may change
// `detail.thread`.
//
// SpecularChat.ready(fn), .followTheme(), .markdown(text), .escape(text)
// and .freshThreadId() are the helpers the elements use. Colours are CSS
// variables named as `colors()` names them, `--sc-<name>` (chat.css).
(() => {
  if (window.SpecularChat) return;
  const here = document.currentScript && document.currentScript.src;
  if (here) {
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = new URL("chat.css", here).href;
    document.head.prepend(link);
  }

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
    const flush = () => { if (para.length) out.push(`<p>${inline(para.join(" "))}</p>`); para = []; };
    const close = () => { if (list) out.push(`</${list}>`); list = null; };
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
      const item = /^\s*[-*]\s+(.*)$/.exec(line) || /^\s*\d+[.)]\s+(.*)$/.exec(line);
      if (heading) {
        flush(); close();
        const level = Math.min(heading[1].length, 4);
        out.push(`<h${level}>${inline(heading[2])}</h${level}>`);
      } else if (item) {
        flush();
        const kind = /^\s*[-*]/.test(line) ? "ul" : "ol";
        if (list !== kind) { close(); out.push(`<${kind}>`); list = kind; }
        out.push(`<li>${inline(item[1])}</li>`);
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

  // The appearance, and the colours and font the active layout renames.
  function followTheme() {
    ready((specular) => {
      specular.subscribe("theme", (theme) => {
        document.documentElement.classList.toggle("dark", Boolean(theme && theme.appearance === "dark"));
      });
      specular.subscribe("look", (look) => {
        const root = document.documentElement.style;
        for (const [name, value] of Object.entries((look && look.colors) || {})) {
          if (typeof value === "string") root.setProperty(`--sc-${name}`, value);
        }
        if (look && look.font) root.setProperty("--sc-font", `"${look.font}", -apple-system, system-ui, sans-serif`);
      });
    });
  }

  const freshThreadId = () =>
    `thread_${Date.now().toString(36)}${Math.floor(Math.random() * 1e8).toString(36)}`;

  // Lucide's glyphs, as the panel draws them.
  const PATHS = {
    plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
    archive: '<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/>',
    x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    down: '<path d="m6 9 6 6 6-6"/>',
    up: '<path d="m5 12 7-7 7 7"/><path d="M12 19V5"/>',
    check: '<path d="M20 6 9 17l-5-5"/>',
    file: '<path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"/><path d="M14 2v5a1 1 0 0 0 1 1h5"/>',
    code: '<path d="m16 18 6-6-6-6"/><path d="m8 6-6 6 6 6"/>',
    folder: '<path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/>',
  };
  const icon = (name) => `<svg class="sc-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${PATHS[name]}</svg>`;

  // A popup list under (or over) `anchor`: rows of {label, detail, checked,
  // pick}. A click anywhere else closes it.
  function menu(anchor, rows, upward) {
    document.querySelectorAll(".sc-menu").forEach((open) => open.remove());
    const box = document.createElement("div");
    box.className = "sc-menu";
    rows.forEach((row) => {
      const button = document.createElement("button");
      button.innerHTML = `<span class="check">${row.checked ? icon("check") : ""}</span><span>${escape(row.label)}${row.detail ? `<small>${escape(row.detail)}</small>` : ""}</span>`;
      button.addEventListener("click", () => { box.remove(); row.pick(); });
      box.append(button);
    });
    document.body.append(box);
    const at = anchor.getBoundingClientRect();
    box.style.left = `${Math.max(4, Math.min(at.left, innerWidth - box.offsetWidth - 4))}px`;
    box.style.top = upward ? `${Math.max(4, at.top - box.offsetHeight - 4)}px` : `${at.bottom + 4}px`;
    setTimeout(() => addEventListener("pointerdown", function away(event) {
      if (box.contains(event.target)) return;
      box.remove();
      removeEventListener("pointerdown", away, true);
    }, true));
  }

  const fileUrl = (path) => (/^[a-z]+:/i.test(path) ? path : `file://${encodeURI(path)}`);
  function message(m) {
    const images = (m.images || []).length
      ? `<div class="thumbs">${m.images.map((p) => `<img src="${escape(fileUrl(p))}">`).join("")}</div>` : "";
    if (m.role === "user") return `<div class="msg user">${images}${escape(m.text)}</div>`;
    const cards = (m.artifacts || []).map((card) =>
      `<div class="card"><b>${card.created ? "Made" : "Edited"} ${escape(card.label)}</b> <span>${escape(card.detail)}</span></div>`);
    return `<div class="msg agent">${markdown(m.text)}${images}${cards.join("")}</div>`;
  }

  class SpecularThread extends HTMLElement {
    static get observedAttributes() { return ["thread"]; }
    connectedCallback() { this.follow(); }
    disconnectedCallback() { this.stop(); }
    attributeChangedCallback() { if (this.isConnected) this.follow(); }
    stop() { if (this.off) this.off(); this.off = null; }
    follow() {
      const id = this.getAttribute("thread");
      if (id === this.following && this.off) return;
      this.stop();
      this.following = id;
      this.value = null;
      this.paint();
      this.dispatchEvent(new CustomEvent("thread", { detail: null, bubbles: true }));
      if (!id) return;
      ready((specular) => {
        if (this.following !== id) return;
        this.off = specular.subscribe(`chat/thread/${id}`, (value) => {
          this.value = value;
          this.paint();
          this.dispatchEvent(new CustomEvent("thread", { detail: value, bubbles: true }));
        });
      });
    }
    paint() {
      const t = this.value;
      const near = this.scrollHeight - this.scrollTop - this.clientHeight < 40;
      const hint = (t && t.emptyHint) || this.getAttribute("empty")
        || "Comment on the canvas to queue a draft, or type below and send.";
      const parts = t ? t.messages.map(message) : [];
      if (t && t.streaming) parts.push(message({ role: "agent", text: t.streaming, artifacts: t.making }));
      if (t && t.error) parts.push(`<div class="error">${escape(t.error)}</div>`);
      if (!t || !t.messages.length) parts.push(`<div class="note">${escape(hint)}</div>`);
      const working = Boolean(t && t.running);
      parts.push(`<div class="agent-row"><div class="avatar${working ? " working" : ""}"></div>${t && t.run ? escape(t.run.label) : ""}</div>`);
      this.innerHTML = parts.join("");
      if (near) this.scrollTop = this.scrollHeight;
    }
  }

  // The formats a thread takes; anything else pasted is left to the field.
  const TAKES = ["image/png", "image/jpeg", "image/gif", "image/webp"];

  class SpecularComposer extends HTMLElement {
    static get observedAttributes() { return ["placeholder"]; }
    connectedCallback() {
      if (this.field) return;
      this.images = [];
      this.innerHTML = `<div class="box"><div class="queued"></div><div class="pasted"></div>
        <textarea rows="2"></textarea><div class="row"><div class="chips">
        <span class="chip pill" hidden></span><span class="chip folder" hidden></span></div>
        <button class="sc-ghost model" hidden></button>
        <button class="send" title="Send" disabled>${icon("up")}</button></div></div>`;
      const $ = (selector) => this.querySelector(selector);
      this.field = $("textarea");
      this.attributeChangedCallback();
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
        if (this.take(event.clipboardData ? event.clipboardData.files : [])) event.preventDefault();
      });
      $(".box").addEventListener("click", (event) => { if (event.target === $(".box")) this.field.focus(); });
      $(".send").addEventListener("click", () => { this.send(); this.field.focus(); });
      $(".pasted").addEventListener("click", (event) => {
        const n = event.target.dataset && event.target.dataset.n;
        if (n === undefined) return;
        this.images.splice(Number(n), 1);
        this.thumbs();
      });
      $(".model").addEventListener("click", () => {
        const rows = (this.models || []).map((option) => ({
          label: option.name, detail: option.description, checked: option.chosen,
          pick: () => window.specular.run("chat.model", { name: option.value || option.name }),
        }));
        menu($(".model"), rows, true);
      });
      ready((specular) => specular.subscribe("chat/context", (value) => this.context(value)));
    }
    attributeChangedCallback() {
      if (this.field) this.field.placeholder = this.getAttribute("placeholder") || "Add or edit…";
    }
    focus() { if (this.field) this.field.focus(); }
    ready() { this.querySelector(".send").disabled = !(this.field.value.trim() || this.images.length); }
    queue(texts) {
      const queued = this.querySelector(".queued");
      if (queued) queued.innerHTML = (texts || []).map((t) => `<span>${escape(t)}</span>`).join("");
    }
    thumbs() {
      this.querySelector(".pasted").innerHTML = this.images
        .map((image, n) => `<div class="thumb"><img src="${image.data}"><button data-n="${n}" title="Remove image">×</button></div>`)
        .join("");
      this.ready();
    }
    take(files) {
      const images = [...files].filter((file) => TAKES.includes(file.type));
      for (const file of images) {
        const reader = new FileReader();
        reader.onload = () => { this.images.push({ type: file.type, data: reader.result }); this.thumbs(); };
        reader.readAsDataURL(file);
      }
      return images.length > 0;
    }
    context(value) {
      if (!value) return;
      const chip = (el, glyph, text, tip) => {
        el.hidden = !text;
        el.innerHTML = text ? `${icon(glyph)}<span>${escape(text)}</span>` : "";
        el.title = tip || "";
      };
      const pill = value.pill || {};
      chip(this.querySelector(".pill"), pill.kind === "dom" ? "code" : "file", pill.label);
      chip(this.querySelector(".folder"), "folder", value.folder,
        value.folderPath ? `Changes are written to ${value.folderPath}` : "");
      this.models = (value.model && value.model.options) || [];
      const model = this.querySelector(".model");
      model.hidden = !value.model;
      model.title = "Model for the next run";
      model.innerHTML = `${escape(value.model ? value.model.label : "")}${icon("down")}`;
    }
    send() {
      const text = this.field.value.trim();
      const images = this.images;
      if (!text && !images.length) return;
      const detail = { text, images, thread: this.getAttribute("thread") || undefined };
      if (!this.dispatchEvent(new CustomEvent("send", { detail, cancelable: true, bubbles: true }))) return;
      this.field.value = "";
      this.field.style.height = "auto";
      this.images = [];
      this.thumbs();
      ready((specular) => specular.run("chat.send", detail).catch((error) => console.error(error)));
    }
  }

  // The whole panel thread: header, transcript and composer, for one
  // thread the element keeps itself.
  class SpecularChat extends HTMLElement {
    static get observedAttributes() { return ["thread", "closable"]; }
    connectedCallback() {
      if (this.log) return;
      this.innerHTML = `<header><div class="title"><button class="sc-ghost switch" title="Switch thread">
        <span>New thread</span>${icon("down")}</button></div>
        <button class="sc-ghost act new" title="New thread">${icon("plus")}</button>
        <button class="sc-ghost act archive" title="Close thread">${icon("archive")}</button>
        <button class="sc-ghost act close" title="Close" hidden>${icon("x")}</button></header>
        <specular-thread></specular-thread><specular-composer></specular-composer>`;
      const $ = (selector) => this.querySelector(selector);
      this.log = $("specular-thread");
      this.composer = $("specular-composer");
      this.log.addEventListener("thread", (event) => {
        const t = event.detail;
        if (t) this.seen = true;
        $(".switch span").textContent = (t && t.title) || "New thread";
        this.composer.setAttribute("placeholder", (t && t.placeholder) || "Add or edit…");
        this.composer.queue(t && t.queued);
      });
      // A thread seen once that reads null now was closed, or is on
      // another canvas: the next message starts a new one here.
      this.composer.addEventListener("send", (event) => {
        if (this.log.value || !this.seen) return;
        this.choose(freshThreadId());
        event.detail.thread = this.getAttribute("thread");
      });
      $(".switch").addEventListener("click", () => {
        const threads = (this.threads && this.threads.threads) || [];
        const current = this.getAttribute("thread");
        const rows = threads.map((t) => ({
          label: t.title || "New thread", detail: t.running ? "Working…" : "", checked: t.id === current,
          pick: () => this.choose(t.id),
        }));
        if (!rows.length) rows.push({ label: "No other threads", pick: () => {} });
        menu($(".switch"), rows, false);
      });
      $(".new").addEventListener("click", () => this.choose(freshThreadId()));
      $(".archive").addEventListener("click", () => {
        const id = this.getAttribute("thread");
        if (id && this.log.value) window.specular.run("chat.close", { id });
        this.choose(freshThreadId());
      });
      $(".close").addEventListener("click", () => this.dispatchEvent(new CustomEvent("close", { bubbles: true })));
      ready((specular) => specular.subscribe("chat/threads", (value) => { this.threads = value; }));
      this.attributeChangedCallback();
    }
    attributeChangedCallback() {
      if (!this.log) return;
      const id = this.getAttribute("thread");
      if (id && id !== this.log.getAttribute("thread")) {
        this.seen = false;
        this.log.setAttribute("thread", id);
        this.composer.setAttribute("thread", id);
      }
      this.querySelector(".close").hidden = !this.hasAttribute("closable");
    }
    choose(id) {
      this.setAttribute("thread", id);
      this.dispatchEvent(new CustomEvent("thread-change", { detail: { thread: id }, bubbles: true }));
      this.composer.focus();
    }
  }

  customElements.define("specular-thread", SpecularThread);
  customElements.define("specular-composer", SpecularComposer);
  customElements.define("specular-chat", SpecularChat);
  window.SpecularChat = Object.freeze({ markdown, escape, ready, followTheme, freshThreadId });
})();
