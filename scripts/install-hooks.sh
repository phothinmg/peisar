#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

git config core.hooksPath .githooks
chmod +x .githooks/commit-msg

echo "Git hooks installed from .githooks"
echo "Active hooksPath: $(git config --get core.hooksPath)"
