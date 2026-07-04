#!/bin/bash
cd "$(dirname "$0")"
cargo build --target wasm32-unknown-unknown --release
echo "WASM plugin built: $(ls -lh target/wasm32-unknown-unknown/release/*.wasm 2>/dev/null || echo 'BUILD FAILED')"
