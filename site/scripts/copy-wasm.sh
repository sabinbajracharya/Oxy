#!/bin/bash
set -euo pipefail
mkdir -p public/wasm
cp -r ../playground/pkg/* public/wasm/
echo "WASM files copied to public/wasm/"
