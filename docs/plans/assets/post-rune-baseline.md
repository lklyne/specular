# Kit release baseline after the Rune spike

Phase 1 of [the follow-ups plan](../post-rune-followups.md), 2026-11-25. The numbers phases 3 and 4 compare against. Kit shell only, release CEF bundle built from `claude/post-rune-followups` at `c5cdb079` (the teardown fix in), the Kit cases of the Rune spike's S6 bench run the way S6 ran them.

## Conditions

- M3 Max, built-in ProMotion display, macOS 27.0, on AC (100%, charged) for the whole run. Window 1600x1000 at scale 2, floating (bench windows float above the others), display kept awake with `caffeinate -d` and Amphetamine.
- Not quiet. A VM ran throughout (75 to 93% of a core), with the Claude desktop app (up to 80%), several `claude` CLIs, Creative Cloud's UI helper and a `workerd`/`esbuild` dev server. Another agent's `rustc` was compiling in a separate worktree when the run began: load average 20 at the start, 4.7 at the end. The pan-9 runs came first, under the heaviest load. `ps-before.txt` and `ps-after.txt` were kept with the run folder.
- Every run exited 0. No run printed a display-link failure.

## Numbers

Median [min to max] of three runs. S6's Kit column (same Mac and fixtures, 2026-11-25, also not quiet) is given for drift.

| Case | Measure | Baseline | S6 Kit | Drift |
|---|---|---|---|---|
| Pan, 9 pages | p50 / p95 / p99 ms | 8.33 / 9.39 / 15.73 [15.68 to 15.84] | 8.33 / 9.83 / 15.57 | none |
| | long frames (>12.5 ms) of ~720 | 16 [15 to 16] | 18 [11 to 19] | none |
| | main thread ms/frame | 1.28 [1.22 to 1.31] | 1.25 | none |
| | pump p95 ms; pump ms/frame p95 | 0.20; 0.25 | 0.25; 0.38 | within range |
| Pan, 20 pages | p50 / p95 / p99 ms | 8.33 / 9.11 / 15.36 [9.27 to 15.77] | 8.33 / 8.73 / 9.20 [8.98 to 16.03] | p99 median up; same bimodal spread as S6 (one clean run, two with 8 and 13 long frames) |
| | main thread ms/frame | 1.38 [1.10 to 1.40] | 1.29 | +7%, ranges overlap |
| | pump p95 ms; pump ms/frame p95 | 0.14; 0.29 | 0.19; 0.29 | none |
| Pan, 40 pages | p50 / p95 / p99 ms | 8.32 / 8.75 / 9.24 [8.98 to 9.68] | 8.34 / 9.03 / 9.38 | none |
| | main thread ms/frame | 1.21 [0.88 to 1.38] | 1.32 | none |
| | pump p95 ms; pump ms/frame p95 | 0.14; 0.34 | 0.14; 0.39 | none |
| Idle, 20 still local pages, 30 s | main loop wakeups/s | 175.2 [174.8 to 176.8] | 176.0 | none |
| | browser process CPU | 3.2% [2.9 to 3.3] | 2.9% | within noise |
| | main thread CPU | 1.8% | 1.7% | none |
| | CEF pumps/s; frames drawn | 30.3; 0 | 30.3; 0 | none |
| Idle, 20 live sites, 30 s | browser process CPU; wakeups/s | 19.1%; 992 | 18.8%; 1,031 | none |
| Memory, 20 live sites, 25 s in | browser + GPU process footprint | 3,007 MB [2,974 to 3,096] | 3,009 MB | none |
| | whole tree | 4,876 MB | 4,883 MB | none |
| Key to present, flip page, 60 fps pages | p50 / p95 ms | 8.30 / 16.48 [8.54 to 19.41] | 8.37 / 25.72 [17.98 to 32.95] | p95 lower, ranges overlap; noisy at 40 keys a run |

Reading it: nothing moved beyond S6's own run-to-run spread. Two things are stable enough to aim at. The Kit wakes its main run loop 175 times a second with nothing moving and draws nothing (phase 4's target). Pan 9 has a steady 15 to 16 long frames and p99 of 15.7 ms in all six runs across S6 and here, while 40 pages pans cleaner; pan 20 flips between clean and ~10 long frames. The key-to-present probe ends after the Kit's present returns (see ADR 0053's "Measured"); every run, here and in S6, also has one sample of about 1.25 s, not looked into.

## Procedure

The bench tools live only at the tag `archive/rune-spike`. From the repo root:

```sh
git worktree add ../telescope-rune-archive archive/rune-spike   # detached; read-only use
cd native
export CEF_PATH="$HOME/.local/share/cef"
cargo build --release -p specular-shell --features cef
cargo build --release -p specular-bench
crates/specular-cef/scripts/bundle-macos.sh release
cd ..
pmset -g batt        # record: AC or battery
ARCHIVE=../telescope-rune-archive caffeinate -d bash kit-baseline.sh OUT_DIR
git worktree remove ../telescope-rune-archive
```

Run on AC, display awake, the window uncovered, hands off; the fixtures `static-*.canvas` load live sites, so the network must be up. `kit-baseline.sh` is below (it was run from a scratch file; copy it out to rerun). It is `bench-s6.sh` from the archive cut to the Kit cases, with the Kit binary pointed at this checkout's bundle and the archive's `s6_run.py`, `s6_table.py`, `still-20.canvas`, `flip.canvas`/`flip.html` and `scripts/s6/flip.txt` used in place. It writes one JSON a run and prints the tables this page was made from. About 10 minutes.

```sh
#!/usr/bin/env bash
# The Kit-only cases of the Rune spike's S6 bench, against a release CEF
# bundle of the Kit built from the current checkout.
#   ARCHIVE=../telescope-rune-archive kit-baseline.sh OUT_DIR
set -euo pipefail
out="${1:?usage: kit-baseline.sh OUT_DIR}"
reps="${REPS:-3}"
native="$(git rev-parse --show-toplevel)/native"
spike="$(cd "${ARCHIVE:?set ARCHIVE to the archive/rune-spike worktree}" && pwd)/native/bakeoff/rune-shell"
mkdir -p "$out"; out="$(cd "$out" && pwd)"
KIT="$native/target/release/specular.app/Contents/MacOS/specular"
export BENCH="$native/target/release/specular-bench"
RUN="python3 $spike/tools/s6_run.py"
WINDOW=1600x1000
WARMUP=20000
PAN_MS=6000
ps -Ao pid,pcpu,pmem,comm -r > "$out/ps-before.txt"
{ sysctl -n machdep.cpu.brand_string; sw_vers -productVersion; pmset -g batt; uptime; git -C "$native" rev-parse HEAD; } > "$out/machine.txt"
kit() { local name="$1"; shift; $RUN "$out/$name.json" -- "$KIT" --window "$WINDOW" --paint-policy full-rate "$@"; }
cd "$native"
for pages in 9 20 40; do
  for i in $(seq "$reps"); do
    kit "pan-$pages-kit-$i" --bench slow-pan --bench-duration-ms "$PAN_MS" --warmup-ms "$WARMUP" "fixtures/static-$pages.canvas"
  done
done
for i in $(seq "$reps"); do
  kit "idlestill-20-kit-$i" --bench idle --bench-duration-ms 30000 --warmup-ms 10000 "$spike/fixtures/still-20.canvas"
done
for i in $(seq "$reps"); do
  RSS_AT=25 kit "idle-20-kit-$i" --bench idle --bench-duration-ms 30000 --warmup-ms "$WARMUP" fixtures/static-20.canvas
done
for i in $(seq "$reps"); do
  space="$(mktemp -d)"
  cp "$spike/fixtures/flip.canvas" "$spike/fixtures/flip.html" "$space/"
  SPECULAR_NATIVE_CONFIG_DIR="$space/config" SPECULAR_FLOAT_WINDOW=1 \
  SPECULAR_SHELL_SCRIPT_FILE="$spike/scripts/s6/flip.txt" \
    $RUN "$out/flip-60-kit-$i.json" -- "$KIT" --window "$WINDOW" "$space/flip.canvas" || true
  rm -rf "$space"
done
ps -Ao pid,pcpu,pmem,comm -r > "$out/ps-after.txt"
{ pmset -g batt; uptime; } >> "$out/machine.txt"
python3 "$spike/tools/s6_table.py" "$out"
```

