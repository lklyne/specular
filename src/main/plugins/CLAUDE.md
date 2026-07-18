# Entity-renderer plugins

This folder is the **main-side** of Specular's entity-renderer registry. The matching React components live in `src/renderer/canvas-bg/entity-renderers/`. A complete plugin spans both — the layer rule (`src/renderer/` cannot import from `src/main/`) means you can't co-locate them.

## How dispatch works

Every file entity asks the registry "which plugin claims me?" The answer is a string `rendererTag` that gets broadcast as part of the scene data:

```
File entity created/updated  →  buildFileEntitySceneEntity (file-entity-state.ts)
                              →  getRendererTagFor(entity)         [main]
                              →  scene entity carries rendererTag
                              →  IPC broadcast to renderer
                              →  RendererSwitch picks the React component  [renderer]
```

Dispatch is **first match wins** by registration order, with `priority` as the override. A throwing `claims()` predicate is logged and treated as "did not claim" — one buggy plugin can't blank out every file behind it.

## The two kinds

`EntityRendererClaim` is a discriminated union:

- **`InlineRendererClaim`** — content renders inside the canvas DOM as a React component. Markdown, image, video are all inline. No process boundary, synchronous render.
- **`WcvPageRendererClaim`** — content loads inside its own Electron `WebContentsView` (a separate renderer process). The plugin provides `resolveUrl(entity)`; the host eventually creates the WCV pointed at it. Component-render is the only `wcv-page` plugin today.

Both kinds share one registry because the dispatch key — file extension — is shared. The discriminated union enforces that `wcv-page` carries `resolveUrl` and `inline` does not.

## How to add an inline renderer

1. **Add the tag** to `EntityRendererTag` in `registry.ts`.
2. **Create the claim file** under `builtin/your-render.ts`:
   ```ts
   import type { InlineRendererClaim } from '../registry'
   export const yourRenderPlugin: InlineRendererClaim = {
     id: 'specular.your-renderer',
     kind: 'inline',
     rendererTag: 'your-tag',
     // True if the renderer reacts to canEdit (markdown, video); false if
     // it ignores it (image, placeholder).
     editable: true,
     claims: (entity) => /\.your-ext$/i.test(entity.file),
   }
   ```
3. **Register it** in `index.ts` by adding `yourRenderPlugin` to the `builtIns` array (alphabetically — order doesn't matter, that's what `priority` is for).
4. **Create the React component** at `src/renderer/canvas-bg/entity-renderers/YourRenderer.tsx`. Each renderer owns its own state (fetch, debounce, focus); see `MarkdownInlineRenderer.tsx` for the pattern.
5. **Add a case** to `RendererSwitch.tsx` in the `switch (tag)` block.

Also update `PanelFileType` in `src/shared/types.ts` and `FileEntityPane.tsx` if your renderer wants its own inspector pane.

## How to add a wcv-page renderer

Same five steps, except:

- The claim has `kind: 'wcv-page'` and **must** carry `resolveUrl(entity) → URL | null`. Returning `null` tells the host to render a placeholder.
- The renderer-side component is a **placeholder** shown while the WCV materializes (or when no URL can be resolved). See `ComponentPlaceholderRenderer.tsx`.
- The actual `WebContentsView` lifecycle is **plugin-specific runtime**, not part of the registry contract. For component-render, that lives in `src/main/runtime/dev-server-manager.ts` (which spawns `vite dev` per connected repo) and `src/main/runtime/component-page-factory.ts` (which mounts a WCV per `.tsx`/`.jsx` file entity, reconciled from the layout pass). A new wcv-page plugin should put its lifecycle code in a sibling file under `runtime/`.

## Conventions

- **Priority.** Default 0. Set higher when your file pattern is more specific than another plugin's (e.g. a `.foo.json` claim beating a hypothetical generic `.json` plugin).
- **Metadata is namespaced.** Per-instance plugin data on a file entity goes in `metadata.<plugin-id>.*` (e.g. `metadata.componentRender.{repoId, repoRelativePath}`). The serializer round-trips `metadata` on file nodes; document new keys in `docs/file-formats.md`.
- **Plugin IDs are not persisted.** A renderer is recovered from the file at load time, never stored. This is deliberate — file-on-disk is the source of truth, and the renderer follows.
- **Layer rule.** Renderer-side dispatch is by string tag broadcast over IPC. Never import `registry.ts` from `src/renderer/`; the `RendererSwitch` already knows everything it needs.

## When this folder grows

If a third rendering mechanism shows up (native overlay, Three.js scene, PDF viewer), `EntityRendererKind` grows and `RendererSwitch` grows with it. At three kinds it's still cheap; at five revisit whether the unified registry still pays for itself.
