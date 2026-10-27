#!/usr/bin/env bash
# Builds the viewer's WebAssembly (native/crates/specular-web) into web/pkg,
# which index.html loads. Needs the wasm32 target and the wasm-bindgen CLI at
# the version native/Cargo.toml pins:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version 0.2.129 --locked
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
native="$here/../native"
cargo build --manifest-path "$native/Cargo.toml" --profile web -p specular-web \
  --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir "$here/pkg" \
  "$native/target/wasm32-unknown-unknown/web/specular_web.wasm"
if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
    -o "$here/pkg/specular_web_bg.wasm" "$here/pkg/specular_web_bg.wasm"
fi
ls -l "$here/pkg"
