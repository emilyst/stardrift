#!/bin/sh

echo "Building for WASM target..."

rustup target add wasm32-unknown-unknown

RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build \
  --profile wasm \
  --target wasm32-unknown-unknown

if ! command -v wasm-bindgen > /dev/null; then
    echo "Installing wasm-bindgen-cli..."
    cargo install wasm-bindgen-cli
fi

mkdir -p out

wasm-bindgen \
    --no-typescript \
    --target web \
    --out-dir ./out/ \
    --out-name "many_body_simulation" \
    ./target/wasm32-unknown-unknown/wasm/many_body_simulation.wasm

gzip --keep --force ./out/many_body_simulation_bg.wasm

cp index.html ./out/
