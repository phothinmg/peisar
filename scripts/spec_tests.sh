#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

is_valid=true

DES_DIR="spec-tests/nodejs/peisar-core"
PACKAGE_JSON="$DES_DIR/package.json"
NODE_DIR="spec-tests/nodejs"

# JSON Files
jd="spec-tests/failed_records"
bs="basic_syntax_v0_31_2"
sb="simple_blocks_v0_31_2"
bc="block_containers_v0_31_2"
ht="html_blocks_v0_31_2"
il="inline_parsing_v0_31_2"
json_files=("$jd/${bs}_r.json" "$jd/${bs}_n.json" "$jd/${sb}_r.json" "$jd/${sb}_n.json" "$jd/${bc}_r.json" "$jd/${bc}_n.json" "$jd/${ht}_r.json" "$jd/${ht}_n.json" "$jd/${il}_r.json" "$jd/${il}_n.json")

is_empty_brackets() {
    local content
    content=$(<"$1")
    [[ "$content" =~ ^[[:space:]]*\[[[:space:]]*\][[:space:]]*$ ]]
}

# Build Peisar Core

[ -d "$DES_DIR" ] && rm -rf "$DES_DIR"

npx napi build --platform --features npm --config-path napi.config.json --output-dir "$DES_DIR"

cat << 'EOF' > "$PACKAGE_JSON"
{
  "name": "peisar-core",
  "main": "index.js",
  "types": "index.d.ts",
  "type": "commonjs"
}
EOF

# Testing Rust Surface
# For Commonmark Spec
cargo test --test common_mark basic_syntax_v0_31_2
sleep 1
cargo test --test common_mark simple_blocks_v0_31_2
sleep 1
cargo test --test common_mark block_containers_v0_31_2
sleep 1
cargo test --test common_mark html_blocks_v0_31_2
sleep 1
cargo test --test common_mark inline_parsing_v0_31_2
# For GFM
sleep 1
cargo test --test gfm

# Testing NodeJs Surface
# For Commonmark Spec
cd "$NODE_DIR"
npm test -- --test-name-pattern="Basic syntax Tests" common_mark.test.js
sleep 1
npm test -- --test-name-pattern="Simple Block Elements Tests" common_mark.test.js
sleep 1
npm test -- --test-name-pattern="Block Containers Tests" common_mark.test.js
sleep 1
npm test -- --test-name-pattern="HTML blocks Tests" common_mark.test.js
sleep 1
npm test -- --test-name-pattern="Inline parsing Tests" common_mark.test.js
# For GFM
npm test -- --test-name-pattern="GFM examples" gfm.test.js

# Go back to repo root
sleep 3
cd "$repo_root"
for file in "${json_files[@]}"; do
    if ! is_empty_brackets "${file}"; then
        echo "Error: expected [] in ${file}" >&2
        exit 1
    fi
done

echo "All spec tests are passed!!!"
exit 0
