#!/usr/bin/env bash
# Makes the web viewer's fonts: the bundled ones cut down to Latin, Greek,
# Cyrillic and the symbols text and code use, with every OpenType feature
# and the hinting kept, so text sets as it does on the desktop. The browser
# has no system fonts to fall back on, so a script outside these ranges was
# never going to draw there. Needs fonttools: pip install fonttools.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
ranges="U+0000-024F,U+0259,U+02B0-036F,U+0370-03FF,U+0400-052F,U+1E00-1EFF,\
U+2000-206F,U+20A0-20CF,U+2100-23FF,U+2460-24FF,U+2500-27BF,U+2B00-2BFF,\
U+FB00-FB06,U+FEFF,U+FFFD"
for font in "$here"/../*.ttf; do
  pyftsubset "$font" --unicodes="$ranges" --layout-features='*' \
    --output-file="$here/$(basename "$font")"
done
