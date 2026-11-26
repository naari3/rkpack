#!/usr/bin/env bash
# macOS 用に universal (arm64 + x86_64) の rkpack.app を作って dist/ に zip で固める。
# 使い方: scripts/package-macos.sh
# 前提: rustup target add aarch64-apple-darwin x86_64-apple-darwin, cargo install cargo-bundle
# 出力: dist/rkpack-v<version>-universal-apple-darwin.zip (中身は rkpack.app と packaging/macos/*.txt)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

version="$(cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "rkpack"))')"

# .app の雛形は arm64 で作り、中のバイナリを universal に差し替える
cargo bundle --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

app="target/aarch64-apple-darwin/release/bundle/osx/rkpack.app"
bin="$app/Contents/MacOS/rkpack"
lipo -create \
    target/aarch64-apple-darwin/release/rkpack \
    target/x86_64-apple-darwin/release/rkpack \
    -output "$bin"
lipo -info "$bin"

# 差し替えで署名が合わなくなるので ad-hoc で署名し直す (arm64 は未署名だと起動できない)
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"

mkdir -p dist
zip="dist/rkpack-v$version-universal-apple-darwin.zip"
rm -f "$zip"

# zip の直下に rkpack.app と初回起動の手順書を並べる
staging="target/macos-package"
rm -rf "$staging"
mkdir -p "$staging"
ditto "$app" "$staging/rkpack.app"
cp packaging/macos/*.txt "$staging/"
ditto -c -k --sequesterRsrc "$staging" "$zip"
echo "created: $zip"
