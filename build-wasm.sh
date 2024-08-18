#!/usr/bin/bash

set -e

cd emu-wasm
wasm-pack build

echo "Patching package.json..."
jq '.files[.files| length] |= . + "emu_wasm_bg.wasm.d.ts"' pkg/package.json | sponge pkg/package.json
cd ..
cd web
rm -rf node_modules
pnpm i
