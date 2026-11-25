#!/bin/sh
# Draws one canvas with the reference and with the spike at one camera and
# prints how the two PNGs differ below the toolbar.
#
#   tools/compare.sh OUT_DIR TAG CAMERA [CANVAS]
#
# Needs both binaries built (see README.md) and, with no GPU,
# VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json.
set -eu
out=$1 tag=$2 camera=$3
here=$(cd "$(dirname "$0")/.." && pwd)
native=$(cd "$here/../.." && pwd)
canvas=${4:-$native/fixtures/kitchen-sink.canvas}
"$native/target/debug/specular-reference" --snapshot "$out/ref-$tag.png" \
    --snapshot-size 1200x800 --snapshot-scale 1 --snapshot-camera "$camera" "$canvas" 2>/dev/null
"$here/target/debug/rune-shell" "$canvas" --camera "$camera" --still \
    --snap "$out/rune-$tag.png" --size 1200x800 --scale 1 --at 0.2 2>&1 | grep "^specular"
python3 "$here/tools/diff.py" "$out/ref-$tag.png" "$out/rune-$tag.png" --skip-top 48 \
    --out "$out/diff-$tag.png"
