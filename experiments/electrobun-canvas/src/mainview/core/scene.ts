import type { Rect } from "../shared/geometry";

// Minimal entity model. Re-derived from the main repo's PersistedPageEntity /
// PersistedTextEntity, but flattened: a single `frame` rect and a single `z`,
// instead of the persisted/scene two-layer split (which exists in Specular only
// because main owns truth and broadcasts a derived scene to renderers).
//
// The crucial design choice: pages and stickies share ONE z-order. That shared
// ordering is what lets a sticky sit above page A while staying below page B —
// the thing ADR 0014 calls architecturally impossible on Electron.

export interface PageEntity {
  id: string;
  kind: "page";
  url: string;
  frame: Rect;
  z: number;
}

export interface StickyEntity {
  id: string;
  kind: "sticky";
  text: string;
  color: string;
  frame: Rect;
  z: number;
}

export type Entity = PageEntity | StickyEntity;

export interface Scene {
  pages: PageEntity[];
  stickies: StickyEntity[];
}

const allEntities = (scene: Scene): Entity[] => [...scene.pages, ...scene.stickies];

/** Entities sorted back-to-front by their shared z value. */
export const zSorted = (scene: Scene): Entity[] =>
  allEntities(scene).sort((a, b) => a.z - b.z);

const withZ = (scene: Scene, updates: Map<string, number>): Scene => ({
  pages: scene.pages.map((p) => (updates.has(p.id) ? { ...p, z: updates.get(p.id)! } : p)),
  stickies: scene.stickies.map((s) =>
    updates.has(s.id) ? { ...s, z: updates.get(s.id)! } : s,
  ),
});

/**
 * Move an entity one step through the shared z-order by swapping z with its
 * neighbor. Stepping a sticky up/down walks it past pages and other stickies
 * alike — which is how you park it *between* two pages.
 */
export const stepZ = (scene: Scene, id: string, dir: 1 | -1): Scene => {
  const sorted = zSorted(scene);
  const i = sorted.findIndex((e) => e.id === id);
  const j = i + dir;
  if (i < 0 || j < 0 || j >= sorted.length) return scene;
  return withZ(
    scene,
    new Map([
      [sorted[i].id, sorted[j].z],
      [sorted[j].id, sorted[i].z],
    ]),
  );
};

export const bringToFront = (scene: Scene, id: string): Scene => {
  const max = Math.max(...allEntities(scene).map((e) => e.z));
  return withZ(scene, new Map([[id, max + 1]]));
};

export const sendToBack = (scene: Scene, id: string): Scene => {
  const min = Math.min(...allEntities(scene).map((e) => e.z));
  return withZ(scene, new Map([[id, min - 1]]));
};

export const moveEntity = (scene: Scene, id: string, dx: number, dy: number): Scene => {
  const shift = <T extends Entity>(e: T): T =>
    e.id === id ? { ...e, frame: { ...e.frame, x: e.frame.x + dx, y: e.frame.y + dy } } : e;
  return { pages: scene.pages.map(shift), stickies: scene.stickies.map(shift) };
};
