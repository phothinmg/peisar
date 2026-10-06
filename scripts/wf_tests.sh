#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

TEMP_DIR="js_tests_temp"
SOURCE_TEST_FILE="js_tests/peisar.test.txt"
DEST_TEST_FILE="$TEMP_DIR/peisar.test.cjs"

npx napi build --platform --features npm --js index.cjs --config-path napi.config.json --output-dir "$TEMP_DIR"

cp "$SOURCE_TEST_FILE" "$DEST_TEST_FILE"
sleep 3
echo "Running tests."
node --test "$DEST_TEST_FILE"
sleep 3
echo "Finished tests."
rm -r "$TEMP_DIR"
echo "Removed temp dir."