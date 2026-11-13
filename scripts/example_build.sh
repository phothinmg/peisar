#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

DES_DIR="examples/nodejs/peisar"
PACKAGE_FILE="$DES_DIR/package.json"

[ -d "$DES_DIR" ] && rm -rf "$DES_DIR"


npx napi build --platform --features npm --config-path napi.config.json --output-dir "$DES_DIR"

cat << 'EOF' > "$PACKAGE_FILE"
{
  "name": "peisar",
  "main": "index.js",
  "types": "index.d.ts",
  "type": "commonjs"
}
EOF

rm -r "$DTS_FILE"
echo "Removed dts file."