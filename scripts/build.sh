#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

npx napi build --platform --config-path napi.config.json

rm -rf peisar.linux-x64-gnu.node