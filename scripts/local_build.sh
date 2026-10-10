#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

DES_DIR="peisarloc"
DTS_FILE="types.d.ts"
MJS_FILE="$DES_DIR/index.mjs"

cat << 'EOF' > "$DTS_FILE"
export type PeisarEmphasisLevel = "Italic" | "Bold";
export type PeisarTableCellAlignment = "Default" | "Left" | "Center" | "Right";
export type PeisarTableCellAlignments = PeisarTableCellAlignment[];
export type PeisarTaskState = "Unchecked" | "Checked";
EOF

npx napi build --platform --features npm --js index.cjs --config-path napi.config.json --output-dir "$DES_DIR"

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

rm -r "$DTS_FILE"
echo "Removed dts file."