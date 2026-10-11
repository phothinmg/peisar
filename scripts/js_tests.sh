#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

MAIN_DIR="js_tests"
DES_DIR="$MAIN_DIR/peisarloc"
DTS_FILE="types.d.ts"
PACKAGE_FILE="$DES_DIR/package.json"

[ -d "$DES_DIR" ] && rm -rf "$DES_DIR"
[ -f "$DTS_FILE" ] && rm "$DTS_FILE"

cat << 'EOF' > "$DTS_FILE"
export type PeisarEmphasisLevel = "Italic" | "Bold";
export type PeisarTableCellAlignment = "Default" | "Left" | "Center" | "Right";
export type PeisarTableCellAlignments = PeisarTableCellAlignment[];
export type PeisarTaskState = "Unchecked" | "Checked";
EOF

npx napi build --platform --features npm --config-path napi.config.json --output-dir "$DES_DIR"

cat << 'EOF' > "$PACKAGE_FILE"
{
  "name": "peisarloc",
  "main": "index.js",
  "types": "index.d.ts",
  "type": "commonjs"
}
EOF

rm -r "$DTS_FILE"
echo "Removed dts file."

cd "$MAIN_DIR"
npm test