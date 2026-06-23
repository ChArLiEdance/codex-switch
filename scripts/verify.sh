#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

echo "== npm build =="
npm run build

echo "== cargo test =="
(cd src-tauri && cargo test)

echo "== tauri app bundle =="
npm run tauri:build -- --bundles app

echo "Verification complete."
