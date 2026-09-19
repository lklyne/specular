#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

cargo build --release

cp target/release/libnative_page_layer.dylib native_page_layer.node

echo "Built native_page_layer.node"
