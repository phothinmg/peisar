#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

TEMP_DIR="js_tests_temp"
SOURCE_TEST_FILE="scripts/peisar.test.txt"
DEST_TEST_FILE="$TEMP_DIR/peisar.test.cjs"
DTS_FILE="types.d.ts"

cat << 'EOF' > "$DTS_FILE"
export type PeisarEmphasisLevel = "Italic" | "Bold";
export type PeisarTableCellAlignment = "Default" | "Left" | "Center" | "Right";
export type PeisarTableCellAlignments = PeisarTableCellAlignment[];
export type PeisarTaskState = "Unchecked" | "Checked";
EOF

npx napi build --platform --features npm --js index.cjs --config-path napi.config.json --output-dir "$TEMP_DIR"

cp "$SOURCE_TEST_FILE" "$DEST_TEST_FILE"
sleep 3
echo "Running tests."
node --test "$DEST_TEST_FILE"
sleep 3
echo "Finished tests."
rm -r "$TEMP_DIR"
echo "Removed temp dir."
rm -r "$DTS_FILE"
echo "Removed dts file."