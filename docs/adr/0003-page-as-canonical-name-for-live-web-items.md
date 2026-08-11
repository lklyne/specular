# ADR 0003 — `Page` as the canonical name for live web items

**Status:** Accepted
**Implementation:** Landed 2026-05-08 — sweeping rename across `src/shared/types.ts`, `src/main/runtime/`, IPC channels (`canvas-frame-*` → `canvas-page-*`), preload bridges, components (`FrameChrome` → `PageChrome`, etc.), serializer (`serializeFrameToLinkNode` → `serializePageToLinkNode`), toolbar copy ("Add page"), tests, and docs. Files renamed accordingly (e.g. `frame-ipc.ts` → `page-ipc.ts`, `workspace-frames.ts` → `workspace-pages.ts`). The runtime `Page` wrapper in `page-factory.ts` keeps its name and now shares vocabulary with the entity. Device-shell internals (`frameView`, `lastFrameBoundsKey`, `frameColor`, the `frame:` key on `boundScreenBoundsForPage` results, the `frame: false` BrowserWindow option) intentionally retain "frame" — they're the device chassis around a page, not the entity. ADRs 0001 and 0002 are unchanged per their original wording.
**Date:** 2026-05-08
**Supersedes premise of:** the `frame` entity-kind name and the parallel runtime/entity vocabulary that came with it (e.g. `PersistedFrameEntity` ↔ runtime `Page` WCV wrapper).
**Related:** [ADR 0001 — click-to-enter frame focus](./0001-click-to-enter-frame-focus.md), [ADR 0002 — canvas-anchored overlay UI](./0002-canvas-anchored-overlay-ui.md). Both ADRs were authored under the old name and are not being rewritten; their behavior is unchanged.

## Context

Specular's data model originally used three different names for the same concept depending on the layer:

- **Disk format (JSON Canvas spec):** `link` node — a URL-pointing node in `.canvas` files.
- **Runtime / persistence (entity layer):** `frame` — `PersistedFrameEntity`, `entity.kind === 'frame'`, `frameFocus`, `serializeFrameToLinkNode`, etc.
- **Runtime / lifecycle (the `WebContentsView` wrapper):** `Page` — `page-factory.ts`, `findPageById`, the actual Electron-side object that loads, paints, and runs JS.

The entity-side `frame` and the WCV-side `Page` are 1:1 with each other. They were never two concepts; they were one concept with two names because the WCV wrapper landed first and the entity vocabulary was added later under "frame" by analogy with HTML `<iframe>`.

The split was tolerable but had ongoing costs:

1. **Two redundant nouns** for the same thing in code, requiring everyone (and every future agent) to learn the mapping.
2. **Figma vocabulary friction.** Specular's audience is designers and frontend developers (`docs/product.md`). Figma's "Frame" is a *container/artboard* primitive — closer to Specular's `Group` than to a URL viewport. Users brought the Figma mental model and had to unlearn it.
3. **The `Browser mode` view toggle** introduced in 2026-Q1 reads naturally as showing pages-as-tabs — not "frames-as-tabs." The toolbar already half-leaned toward a different vocabulary.
4. **The multi-breakpoint workflow** (`docs/product.md` "common workflows") describes "the same site at phone, tablet, desktop widths." `Page` extends naturally to that ("three pages of stripe.com at three breakpoints"); `Frame` is narrower and required a separate "breakpoint cluster" concept to talk about variants.

Candidates considered: `Frame` (status quo), `Page`, `Site`, `Link`, `URL`, `View`, `Tab`, `Window`, `Web view`, `Embed`. Evaluation matrix:

| Term | Reads as noun | Multi-of-same-URL workflow | Implies live + rendered | No edge collision | No Figma collision | No internal-codebase clash | Spec-aligned |
|---|---|---|---|---|---|---|---|
| **Page** | ✓ | slight weirdness (page = URL) | neutral | ✓ | ✓ | **unifies with runtime `Page`** | ✗ |
| Frame | ✓ | requires separate "variant" noun | yes (iframe heritage) | ✓ | ✗ Figma | ✓ | ✗ |
| Link | ✓ | "three links to stripe" — passive | ✗ | ✗ collides with edges | ✓ | ✓ | ✓ exact spec match |
| URL | ✗ it's a string | ✗ | ✗ | ✓ | ✓ | conflicts with the `url` field on the entity | ✗ |
| Site | ✓ | ✗ a site IS the URL | ✓ | ✓ | ✓ | ✓ | ✗ |
| View | ⚠️ generic | ✓ "phone view, tablet view" | ⚠️ | ✓ | ✓ | ✗✗ overloaded (`bgView`, `aboveView`, view-mode) | ✗ |
| Tab | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ workspace tabs | ✗ |
| Window | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ Electron `BrowserWindow` | ✗ |

`Page` is the only candidate that survives every cut **and** unifies a redundant pair of names already in the codebase. The slight weakness — that *"three pages of stripe.com"* is mildly weird because casually a page = a URL — is handled by giving the variant relationship its own noun (a "breakpoint", or a "breakpoint variant"), not by overloading the entity name.

## Decision

`Page` is the canonical name for the live-web-item entity kind, end-to-end:

- **User-facing** — toolbar says "Add page", sidebar lists pages, docs talk about pages.
- **Component layer** — `<PageChrome>`, `PageBodyLayer`, etc.
- **Runtime / persistence** — `entity.kind === 'page'`, `PersistedPageEntity`, `serializePageToLinkNode`. The runtime WCV wrapper (already called `Page`) is the same concept — no longer a separate noun. (The `frameFocus` variable from ADR 0001 was superseded by `interactivePageId` in ADR 0022 — it was not renamed to `pageFocus`.)
- **JSON Canvas disk format** — still serialized as `type: 'link'` per spec. The serializer is the *only* place the two names meet.

A page is defined as: **a URL rendered at a particular viewport size at a particular position on the canvas.** Multiple pages can share the same URL (multi-breakpoint workflow); they remain independent entities.

ADRs 0001 and 0002 are not rewritten. They use "frame" for historical accuracy. The CONTEXT.md glossary entry for `Page` notes the rename.

## Consequences

**Replaces:**
- The `frame` entity-kind name across `src/main/runtime/`, `src/shared/types.ts`, IPC channel names (`canvas-frame-*` → `canvas-page-*`), preload bridges, and component names. One sweeping rename PR.
- The "frame entity vs page wrapper" mental-model split. They are now one concept with one name.
- The toolbar label "Add Frame" → "Add page".

**Enables:**
- One vocabulary across all four layers (UI / runtime / IPC / persistence) for the most common entity kind.
- A clean home for breakpoint-variant terminology (a *breakpoint* is a property of a page, not a separate entity kind).
- Removes the Figma-Frame mental-model friction for new users.

**Costs:**
- Sweeping rename touching ~100 identifiers, ~5 IPC channel names, type aliases, smoke-test references, and comments. Mechanical but real.
- ADRs 0001 and 0002 retain "frame" terminology; readers of those documents must mentally translate. The CONTEXT.md glossary points them at this ADR.
- The serializer translation (`PersistedPageEntity` ↔ JSON Canvas `link`) remains. The function renames from `serializeFrameToLinkNode` to `serializePageToLinkNode` but the bridge persists. We accept the divergence from spec because spec alignment alone is not enough to outweigh the internal unification win and the Figma-friction cost.

**Out of scope:**
- Renaming JSON Canvas `link` nodes upstream. The spec is what it is; we serialize to it.
- Folding the `Page` runtime wrapper into the entity layer. They remain in their respective process layers (main vs runtime mutation), they just share a name now.
- Renaming `Browser mode` (the view toggle). It already reads correctly with the new vocabulary ("Browser mode shows pages as tabs").

## Migration plan

The rename ships as one PR (rather than incrementally) so partial states don't confuse contributors. Sequence within the PR:

1. `src/shared/types.ts` — `CanvasEntityKind` `'frame'` → `'page'`; type aliases `PersistedFrameEntity` → `PersistedPageEntity`, etc.
2. `src/main/runtime/` — rename all `frame` references in entity state, selection, layout. The existing runtime `Page` (in `page-factory.ts`) keeps its name and now has matching entity vocabulary.
3. IPC channels — `canvas-frame-*` → `canvas-page-*`; update preload bridges and renderer subscribers.
4. Components — `<FrameChrome>` → `<PageChrome>`, `FrameBodyLayer` → `PageBodyLayer`, etc.
5. `serializeFrameToLinkNode` → `serializePageToLinkNode`; serializer continues to emit/read `type: 'link'`.
6. Toolbar copy: "Add Frame" → "Add page"; sidebar labels; cursor labels.
7. Tests — smoke and unit tests follow the rename.
8. Docs — `CLAUDE.md`, `docs/product.md`, `docs/architecture.md`, `docs/file-formats.md` updated. ADRs 0001/0002 left as-is with a brief note.

## Tests

- Smoke: existing `frame`-named scenarios renamed; behavior unchanged.
- Unit: serializer round-trip test verifies `PersistedPageEntity` → `link` node → `PersistedPageEntity`.
- Type checking: `pnpm typecheck` is the primary gate for the rename.
