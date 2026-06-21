import { useCallback, useState } from "react";
import { Canvas } from "./canvas/Canvas";
import { Toolbar } from "./canvas/Toolbar";
import { bringToFront, moveEntity, stepZ, type Scene } from "./core/scene";
import { useCamera } from "./hooks/useCamera";
import { usePanTool } from "./hooks/usePanTool";

// The starting scene is rigged to show the thesis on launch: three overlapping
// pages and a sticky parked at z=1 — above page A (z=0) but below page B (z=2).
// In the A∩B overlap the sticky shows through A and is hidden by B at the same
// time. That single frame is the thing ADR 0014 calls impossible on Electron.
const STICKY_YELLOW = "#ffd84d";
const STICKY_PINK = "#ff9ec4";
const STICKY_BLUE = "#9ed0ff";

const initialScene: Scene = {
  pages: [
    { id: "page-a", kind: "page", url: "https://example.com", z: 0, frame: { x: 140, y: 180, width: 520, height: 360 } },
    { id: "page-b", kind: "page", url: "https://bun.sh", z: 2, frame: { x: 440, y: 300, width: 540, height: 380 } },
    { id: "page-c", kind: "page", url: "https://news.ycombinator.com", z: 4, frame: { x: 820, y: 150, width: 460, height: 360 } },
  ],
  stickies: [
    { id: "sticky-mid", kind: "sticky", text: "above A, below B — drag me into the overlap", color: STICKY_YELLOW, z: 1, frame: { x: 470, y: 330, width: 200, height: 130 } },
    { id: "sticky-front", kind: "sticky", text: "in front of everything", color: STICKY_PINK, z: 5, frame: { x: 250, y: 250, width: 170, height: 110 } },
    { id: "sticky-back", kind: "sticky", text: "behind every page", color: STICKY_BLUE, z: -1, frame: { x: 900, y: 240, width: 170, height: 110 } },
  ],
};

let stickyCounter = 0;

export function App() {
  const { camera, onWheel, panByScreen } = useCamera();
  const panActive = usePanTool();
  const [scene, setScene] = useState<Scene>(initialScene);
  const [selectedId, setSelectedId] = useState<string | null>("sticky-mid");
  const [overlayEnabled, setOverlayEnabled] = useState(false);

  const onMove = useCallback(
    (id: string, dx: number, dy: number) => setScene((s) => moveEntity(s, id, dx, dy)),
    [],
  );
  const onStepZ = useCallback(
    (id: string, dir: 1 | -1) => setScene((s) => stepZ(s, id, dir)),
    [],
  );
  const onEditSticky = useCallback(
    (id: string, text: string) =>
      setScene((s) => ({
        ...s,
        stickies: s.stickies.map((st) => (st.id === id ? { ...st, text } : st)),
      })),
    [],
  );
  const onAddSticky = useCallback(() => {
    const id = `sticky-${++stickyCounter}-${Date.now()}`;
    setScene((s) =>
      bringToFront(
        {
          ...s,
          stickies: [
            ...s.stickies,
            { id, kind: "sticky", text: "new note", color: STICKY_YELLOW, z: 0, frame: { x: 360, y: 360, width: 180, height: 120 } },
          ],
        },
        id,
      ),
    );
    setSelectedId(id);
  }, []);

  return (
    <>
      <Toolbar
        panActive={panActive}
        overlayEnabled={overlayEnabled}
        onAddSticky={onAddSticky}
        onToggleOverlay={() => setOverlayEnabled((v) => !v)}
      />
      <Canvas
        camera={camera}
        scene={scene}
        selectedId={selectedId}
        panActive={panActive}
        overlayEnabled={overlayEnabled}
        onWheel={onWheel}
        onPanByScreen={panByScreen}
        onSelect={setSelectedId}
        onMove={onMove}
        onStepZ={onStepZ}
        onEditSticky={onEditSticky}
      />
    </>
  );
}
