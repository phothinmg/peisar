#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

# Get Commit Type
# A special shell variable used to set the prompt text for the select command
PS3="Select a number for commit type: "


options=("⭐ feat" "🐛 bug" "🎨 modified" "🔒 security" "👕 refactor" "⚠️ deprecated" "🚀 release" "✅ tests" "📝 docs")

select opt in "${options[@]}"
do
    # If user types 1, $opt becomes "Foo"
    if [ -n "$opt" ]; then
        TYPE="$opt"
        break
    else
        echo "Invalid selection. Try again."
    fi
done

# Get commit message
read -p "Enter commit message: " message
# Run linter
# npm run lint
# Run formatter
# npm run fmt
cargo fmt
# Generate coverage report
# npm run coverage
# Get full commit message pattern
# <Type>: <message>
commit_message="$TYPE : $message"
current_branch=$(git branch --show-current)
git add .
git commit -m "$commit_message"
git push origin "$current_branch"

echo "Push $current_branch with $commit_message"