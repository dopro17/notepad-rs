#!/usr/bin/env bash
# Gera notepad-rs.app universal (Apple Silicon + Intel) em target/universal/.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo bundle --release

out=target/universal
rm -rf "$out" && mkdir -p "$out"
cp -R target/release/bundle/osx/notepad-rs.app "$out/"
lipo -create -output "$out/notepad-rs.app/Contents/MacOS/notepad-rs" \
  target/aarch64-apple-darwin/release/notepad-rs \
  target/x86_64-apple-darwin/release/notepad-rs
# Assinatura ad-hoc (uso próprio; para distribuir: Developer ID + notarytool).
codesign --force --deep --sign - "$out/notepad-rs.app"
lipo -info "$out/notepad-rs.app/Contents/MacOS/notepad-rs"
echo "Pronto: $out/notepad-rs.app"
