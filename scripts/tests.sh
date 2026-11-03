#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

MAIN_DIR="tests"
DES_DIR="$MAIN_DIR/peisarloc"
DTS_FILE="types.d.ts"
PEISAR_PACKAGE="$DES_DIR/package.json"
TESTS_PACKAGE="$MAIN_DIR/package.json"
SOURCE_TEST_FILE="scripts/peisar.test.txt"
DEST_TEST_FILE="$MAIN_DIR/peisar.test.js"
CONFIG_YAML_FILE="$MAIN_DIR/config.yml"

# 1. DTS

echo "Write .d.ts file for peisar build"

cat << 'EOF' > "$DTS_FILE"
export type PeisarEmphasisLevel = "Italic" | "Bold";
export type PeisarTableCellAlignment = "Default" | "Left" | "Center" | "Right";
export type PeisarTableCellAlignments = PeisarTableCellAlignment[];
export type PeisarTaskState = "Unchecked" | "Checked";
EOF

# 2. Peisar Build

echo "Build peisar"

npx napi build --platform --features npm --config-path napi.config.json --output-dir "$DES_DIR"

# 3. Package JSON for peisar

sleep 1

echo "Write package.json for peisar"

cat << 'EOF' > "$PEISAR_PACKAGE"
{
  "name": "peisarloc",
  "main": "index.js",
  "types": "index.d.ts",
  "type": "commonjs"
}
EOF

# 4. Package JSON for test

sleep 1

echo "Write package.json for test"

cat << 'EOF' > "$TESTS_PACKAGE"
{
  "name": "tests",
  "type": "commonjs",
  "scripts": {
    "test": "node --test"
  }
}
EOF

# 5. Write config.yml for test

sleep 1

echo "Write config.yml"

cat << 'EOF' > "$CONFIG_YAML_FILE"
name: config
foo:
  bar: 123
  biz: foo
EOF

# 6. Create test JS file

sleep 1

echo "Copy tests file"

cp "$SOURCE_TEST_FILE" "$DEST_TEST_FILE"

# 7. Running tests

sleep 1

echo "Running tests"

cd "$MAIN_DIR"
npm run test

echo "Finished tests."

# 8. Removing DTS and MAIN_DIR

sleep 1

echo "Removing DTS and MAIN_DIR"

cd "$repo_root"
rm -rf "$DTS_FILE"
rm -rf "$MAIN_DIR"

sleep 1

echo "ALL DONE !!"



