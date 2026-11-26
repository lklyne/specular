# S6 run tables (M3 Max, 2026-11-25)

Printed by `native/bakeoff/rune-shell/tools/s6_table.py` from the run folder: median [min-max] over the runs. `kitpan-*` are the Kit's scripted window pans for the animated-chrome case; `shimmer-20`/`noshimmer-20` are Rune's. Release builds, real CEF pages, machine not quiet (see the plan's S6 entry).

## flip-120  (runs: kit 0, rune 3)
| | Kit | Rune |
|---|---|---|
| samples | - | 82.00 [82.00-82.00] |
| unresolved | - | 0.00 [0.00-0.00] |
| key to present p50 ms | - | 12.72 [11.00-14.04] |
| key to present p95 ms | - | 14.29 [12.06-16.03] |
| key to present max ms | - | 929.70 [927.42-934.55] |
| paint to present p50 ms | - | 6.18 [4.41-8.00] |
| paint to present p95 ms | - | 8.90 [6.36-10.62] |

## flip-60  (runs: kit 3, rune 3)
| | Kit | Rune |
|---|---|---|
| samples | 122.00 [122.00-122.00] | 82.00 [82.00-82.00] |
| unresolved | 0.00 [0.00-0.00] | 0.00 [0.00-0.00] |
| key to present p50 ms | 8.37 [8.32-8.39] | 12.10 [9.88-12.80] |
| key to present p95 ms | 25.72 [17.98-32.95] | 14.50 [11.17-15.04] |
| key to present max ms | 1241.14 [1240.82-1258.76] | 1023.38 [933.90-1024.64] |
| paint to present p50 ms | - | 5.55 [2.54-6.27] |
| paint to present p95 ms | - | 8.20 [4.56-8.72] |

## gpu-1  (runs: kit 0, rune 3)
| | Kit | Rune |
|---|---|---|
| GPU after submit mean ms | - | 1.99 [1.88-2.01] |
| GPU after submit p95 ms | - | 2.88 [2.73-2.99] |
| render CPU mean ms | - | 2.743 [2.718-2.910] |

## gpu-4  (runs: kit 0, rune 3)
| | Kit | Rune |
|---|---|---|
| GPU after submit mean ms | - | 1.91 [1.81-2.32] |
| GPU after submit p95 ms | - | 3.11 [2.62-4.97] |
| render CPU mean ms | - | 2.673 [2.545-3.083] |

## idle-20  (runs: kit 3, rune 3)
| | Kit | Rune |
|---|---|---|
| browser process CPU | 0.188 [0.185-0.214] | 0.259 [0.225-0.262] |
| CEF children CPU | 0.329 [0.328-0.346] | 0.312 [0.304-0.317] |
| main thread CPU | 0.107 [0.106-0.118] | 0.149 [0.129-0.151] |
| main loop turns/s | 1031.00 [999.90-1061.60] | 846.00 [735.20-897.80] |
| pumps/s | 341.70 [339.90-356.80] | 312.30 [284.70-325.20] |
| frames drawn | 1459 [1435-2225] | 2386 [2143-2656] |
| footprint browser+GPU MB | 3009.25 [3006.96-3026.68] | 2927.96 [2888.19-2982.49] |
| footprint tree MB | 4883.36 [4865.82-4907.98] | 4756.07 [4714.81-4829.45] |

## idlestill-20  (runs: kit 3, rune 3)
| | Kit | Rune |
|---|---|---|
| browser process CPU | 0.029 [0.026-0.033] | 0.013 [0.013-0.014] |
| CEF children CPU | -0.010 [-0.019-0.002] | 0.006 [0.005-0.011] |
| main thread CPU | 0.017 [0.015-0.019] | 0.007 [0.006-0.007] |
| main loop turns/s | 176.00 [175.80-177.40] | 32.10 [32.00-32.20] |
| pumps/s | 30.30 [30.30-30.40] | 25.40 [25.40-30.30] |
| frames drawn | 0 [0-0] | 16 [15-16] |
| footprint browser+GPU MB | - | - |
| footprint tree MB | - | - |

## kitpan-noshimmer  (runs: kit 3, rune 0)
| | Kit | Rune |
|---|---|---|
| p50 ms | 8.33 [8.33-8.33] | - |
| p95 ms | 8.66 [8.66-9.12] | - |
| p99 ms | 14.61 [13.91-14.99] | - |
| long frames | 12 [9-12] | - |
| fps | 117.90 [117.90-118.40] | - |
| main thread ms/frame | - | - |
| main thread CPU | - | - |
| browser process CPU | - | - |
| CEF children CPU | - | - |
| pump mean ms | - | - |
| pump p95 ms | - | - |
| pump max ms | - | - |
| pump ms/frame p95 | - | - |
| pump ms/s | - | - |
| rune pacing | - |  |

## kitpan-shimmer  (runs: kit 3, rune 0)
| | Kit | Rune |
|---|---|---|
| p50 ms | 8.33 [8.33-8.33] | - |
| p95 ms | 9.31 [9.20-9.45] | - |
| p99 ms | 10.05 [10.01-10.19] | - |
| long frames | 3 [1-3] | - |
| fps | 119.80 [119.50-119.80] | - |
| main thread ms/frame | - | - |
| main thread CPU | - | - |
| browser process CPU | - | - |
| CEF children CPU | - | - |
| pump mean ms | - | - |
| pump p95 ms | - | - |
| pump max ms | - | - |
| pump ms/frame p95 | - | - |
| pump ms/s | - | - |
| rune pacing | - |  |

## noshimmer-20  (runs: kit 0, rune 3)
| | Kit | Rune |
|---|---|---|
| p50 ms | - | 8.37 [8.32-8.42] |
| p95 ms | - | 12.28 [11.04-14.37] |
| p99 ms | - | 14.78 [13.78-17.00] |
| long frames | - | 35 [22-70] |
| fps | - | 114.26 [108.70-116.43] |
| main thread ms/frame | - | 1.94 [1.92-2.12] |
| main thread CPU | - | 0.214 [0.213-0.222] |
| browser process CPU | - | 0.378 [0.371-0.382] |
| CEF children CPU | - | 0.310 [0.280-0.321] |
| pump mean ms | - | 0.06 [0.05-0.07] |
| pump p95 ms | - | 0.16 [0.14-0.17] |
| pump max ms | - | 4.13 [2.47-6.82] |
| pump ms/frame p95 | - | 0.32 [0.24-0.33] |
| pump ms/s | - | 14.90 [14.61-16.92] |
| rune pacing | - | {"display link": 721}; {"display link": 721}; {"display link": 721} |

## pan-20  (runs: kit 3, rune 3)
| | Kit | Rune |
|---|---|---|
| p50 ms | 8.33 [8.32-8.34] | 8.30 [8.30-8.34] |
| p95 ms | 8.73 [8.68-9.37] | 10.49 [10.08-10.68] |
| p99 ms | 9.20 [8.98-16.03] | 13.57 [10.49-13.58] |
| long frames | 2 [1-16] | 11 [0-17] |
| fps | 119.77 [118.34-120.00] | 118.01 [117.47-120.03] |
| main thread ms/frame | 1.29 [1.21-1.36] | 1.90 [1.76-1.92] |
| main thread CPU | 0.149 [0.139-0.154] | 0.215 [0.203-0.215] |
| browser process CPU | 0.292 [0.274-0.301] | 0.375 [0.368-0.390] |
| CEF children CPU | 0.221 [0.185-0.244] | 0.290 [0.269-0.301] |
| pump mean ms | 0.06 [0.06-0.07] | 0.07 [0.05-0.07] |
| pump p95 ms | 0.19 [0.14-0.20] | 0.18 [0.15-0.19] |
| pump max ms | 9.45 [2.89-9.88] | 9.25 [2.36-9.99] |
| pump ms/frame p95 | 0.29 [0.28-0.35] | 0.34 [0.27-0.38] |
| pump ms/s | 17.90 [17.76-19.90] | 18.53 [15.72-19.62] |
| rune pacing | - | {"display link": 721}; {"display link": 721}; {"display link": 721} |

## pan-40  (runs: kit 3, rune 3)
| | Kit | Rune |
|---|---|---|
| p50 ms | 8.34 [8.33-8.34] | 8.31 [8.27-8.33] |
| p95 ms | 9.03 [8.84-9.29] | 10.51 [10.39-10.54] |
| p99 ms | 9.38 [8.93-9.55] | 13.89 [13.14-14.96] |
| long frames | 1 [0-1] | 14 [8-15] |
| fps | 119.81 [119.77-119.99] | 117.62 [117.24-118.52] |
| main thread ms/frame | 1.32 [1.24-1.52] | 1.79 [1.69-1.82] |
| main thread CPU | 0.151 [0.142-0.174] | 0.202 [0.192-0.204] |
| browser process CPU | 0.291 [0.274-0.316] | 0.351 [0.339-0.363] |
| CEF children CPU | 0.374 [0.319-0.425] | 0.385 [0.364-0.407] |
| pump mean ms | 0.05 [0.05-0.07] | 0.05 [0.05-0.06] |
| pump p95 ms | 0.14 [0.14-0.17] | 0.12 [0.12-0.14] |
| pump max ms | 9.38 [8.70-9.84] | 9.76 [7.67-10.02] |
| pump ms/frame p95 | 0.39 [0.35-0.51] | 0.30 [0.26-0.35] |
| pump ms/s | 27.05 [24.72-31.05] | 23.67 [22.95-26.18] |
| rune pacing | - | {"display link": 721}; {"display link": 721}; {"display link": 721} |

## pan-9  (runs: kit 3, rune 3)
| | Kit | Rune |
|---|---|---|
| p50 ms | 8.33 [8.32-8.34] | 8.32 [8.31-8.34] |
| p95 ms | 9.83 [9.34-9.83] | 9.44 [9.43-10.41] |
| p99 ms | 15.57 [15.48-15.77] | 12.55 [12.34-13.23] |
| long frames | 18 [11-19] | 9 [6-13] |
| fps | 118.31 [117.20-119.34] | 118.20 [117.87-118.69] |
| main thread ms/frame | 1.25 [1.18-1.42] | 1.88 [1.82-2.01] |
| main thread CPU | 0.143 [0.133-0.161] | 0.211 [0.207-0.228] |
| browser process CPU | 0.297 [0.273-0.321] | 0.387 [0.375-0.412] |
| CEF children CPU | 0.184 [0.161-0.211] | 0.199 [0.189-0.228] |
| pump mean ms | 0.09 [0.07-0.14] | 0.07 [0.06-0.08] |
| pump p95 ms | 0.25 [0.18-0.30] | 0.18 [0.17-0.23] |
| pump max ms | 6.98 [6.49-10.02] | 3.09 [0.42-8.10] |
| pump ms/frame p95 | 0.38 [0.25-0.39] | 0.29 [0.23-0.33] |
| pump ms/s | 14.76 [13.48-14.83] | 12.61 [10.60-12.86] |
| rune pacing | - | {"display link": 721}; {"display link": 721}; {"display link": 721} |

## popover-40  (runs: kit 0, rune 3)
| | Kit | Rune |
|---|---|---|
| p50 ms | - | 8.33 [8.30-8.36] |
| p95 ms | - | 11.25 [10.39-13.26] |
| p99 ms | - | 16.54 [13.61-16.61] |
| long frames | - | 27 [16-43] |
| fps | - | 115.68 [112.93-117.55] |
| main thread ms/frame | - | 2.04 [1.92-2.07] |
| main thread CPU | - | 0.224 [0.216-0.226] |
| browser process CPU | - | 0.378 [0.369-0.379] |
| CEF children CPU | - | 0.408 [0.399-0.417] |
| pump mean ms | - | 0.05 [0.05-0.06] |
| pump p95 ms | - | 0.13 [0.13-0.13] |
| pump max ms | - | 9.63 [8.68-10.01] |
| pump ms/frame p95 | - | 0.41 [0.29-0.42] |
| pump ms/s | - | 24.28 [22.99-27.01] |
| rune pacing | - | {"display link": 721}; {"display link": 721}; {"display link": 721} |

## shimmer-20  (runs: kit 0, rune 3)
| | Kit | Rune |
|---|---|---|
| p50 ms | - | 8.37 [8.36-8.37] |
| p95 ms | - | 13.15 [12.56-13.22] |
| p99 ms | - | 16.78 [16.44-16.79] |
| long frames | - | 44 [37-49] |
| fps | - | 112.48 [112.37-112.96] |
| main thread ms/frame | - | 2.15 [2.01-2.15] |
| main thread CPU | - | 0.232 [0.218-0.232] |
| browser process CPU | - | 0.396 [0.380-0.400] |
| CEF children CPU | - | 0.306 [0.278-0.325] |
| pump mean ms | - | 0.06 [0.06-0.07] |
| pump p95 ms | - | 0.17 [0.16-0.17] |
| pump max ms | - | 8.66 [3.00-9.78] |
| pump ms/frame p95 | - | 0.32 [0.30-0.36] |
| pump ms/s | - | 15.77 [15.30-16.72] |
| rune pacing | - | {"display link": 721}; {"display link": 721}; {"display link": 721} |
