#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

BUILD_DIR="dist"
MJS_FILE="$BUILD_DIR"/index.mjs
BINARY_FILE="$BUILD_DIR"/peisar.linux-x64-gnu.node
DTS_FILE="types.d.ts"

# 1. Write .d.ts file for napi-rs
cat << 'EOF' > "$DTS_FILE"
// Generated from ci.yml
// napi-rs helper types 
export type PeisarEmphasisLevel = "Italic" | "Bold";
export type PeisarTableCellAlignment = "Default" | "Left" | "Center" | "Right";
export type PeisarTableCellAlignments = PeisarTableCellAlignment[];
export type PeisarTaskState = "Unchecked" | "Checked";
EOF

sleep 3

# 2. Build to build dir
npx napi build --platform --features npm --js index.cjs --config-path napi.config.json --output-dir "$BUILD_DIR"

# 3. Write index.mjs to build dir
cat << 'EOF' > "$MJS_FILE"
// Generated form ci.yml
import { createRequire } from "module";
const require = createRequire(import.meta.url);
const peisar = require("./index.cjs");

export const Peisar = peisar.Peisar;
export const PeisarCache = peisar.PeisarCache;
export const frontmatter = peisar.frontmatter;
export const yamlParser = peisar.yamlParser;
EOF

# 4. Remove binary and .d.ts
rm -rf "$BINARY_FILE"
sleep 3
rm -rf "$DTS_FILE"