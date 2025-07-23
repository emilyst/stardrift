#!/bin/sh

echo "Building for WASM target..."

rustup target add wasm32-unknown-unknown

cargo build \
  --profile wasm \
  --target wasm32-unknown-unknown \
  --features trails

if ! command -v wasm-bindgen > /dev/null; then
    echo "Installing wasm-bindgen-cli..."
    cargo install wasm-bindgen-cli
fi

mkdir -p out

wasm-bindgen \
    --no-typescript \
    --target web \
    --out-dir ./out/ \
    --out-name "stardrift" \
    ./target/wasm32-unknown-unknown/wasm/stardrift.wasm

cp index.html ./out/

gzip --keep --force ./out/*.html
gzip --keep --force ./out/*.js
gzip --keep --force ./out/*.wasm
