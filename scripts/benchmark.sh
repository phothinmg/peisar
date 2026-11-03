#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cd bench

PACKAGE_LOG="package-lock.json"
NODE_MODULES="node_modules"

# --- Function to detect OS and attempt auto-install of jq ---
try_install_jq() {
    echo "jq is missing. Attempting auto-installation..."
    
    local os_family
    os_family=$(uname -s)
    
    case "$os_family" in
        Linux*)
            if [ -f /etc/os-release ]; then
                . /etc/os-release
                case "$ID" in
                    ubuntu|debian)
                        sudo apt-get update && sudo apt-get install -y jq
                        ;;
                    centos|rhel|fedora)
                        sudo yum install -y jq || sudo dnf install -y jq
                        ;;
                    alpine)
                        apk add jq
                        ;;
                    *)
                        return 1 # Unsupported Linux distro for auto-install
                        ;;
                esac
            else
                return 1
            fi
            ;;
        Darwin*)
            if command -v brew &> /dev/null; then
                brew install jq
            else
                return 1 # Homebrew not installed
            fi
            ;;
        *)
            return 1 # Unsupported OS family (e.g., Windows native)
            ;;
    esac
}

# --- Function for Fallback Script Execution ---
run_fallback_bench() {
    echo "Warning: Could not use/install jq. Running fallback benchmark without updating packages..."
    
    echo "Removing old lock and modules if exists"
    [ -d "$NODE_MODULES" ] && rm -rf "$NODE_MODULES"
    [ -f "$PACKAGE_LOG" ] && rm "$PACKAGE_LOG"
    
    echo "Install Packages"
    npm i

    echo "Running benchmark"
    npm run bench
    
    echo "Removing package-lock.json and node_modules"
    [ -d "$NODE_MODULES" ] && rm -rf "$NODE_MODULES"
    [ -f "$PACKAGE_LOG" ] && rm "$PACKAGE_LOG"
    
    echo "Done benchmarking (via Fallback)"
    exit 0
}

# --- Core Logic Validation ---
# If jq is missing, try installing it. If installation fails, run fallback.
if ! command -v jq &> /dev/null; then
    if ! try_install_jq; then
        run_fallback_bench
    fi
fi

# Double check that jq is truly functional now
if ! command -v jq &> /dev/null; then
    run_fallback_bench
fi

# --- Main workflow using jq ---
echo "Check and Update Packages"

packages=("markdown-it" "marked" "micromark" "micromark-extension-gfm" "peisar" "showdown")

for package in "${packages[@]}"; do
    local_version=$(jq -r --arg pkg "$package" '.dependencies[$pkg] | gsub("[~^]"; "")' package.json)
    latest_version=$(npm view "$package" version)
    
    # Handle cases where package doesn't exist in package.json (returns "null")
    if [[ "$local_version" == "null" || "$local_version" != "$latest_version" ]]; then
        echo "Update $package to ^$latest_version"
        jq --arg pkg "$package" --arg ver "^$latest_version" '.dependencies[$pkg] = $ver' package.json > package.json.tmp && mv package.json.tmp package.json
    else
        echo "Passed $package"
    fi
done

echo "Removing old lock and modules if exists"
[ -d "$NODE_MODULES" ] && rm -rf "$NODE_MODULES"
[ -f "$PACKAGE_LOG" ] && rm "$PACKAGE_LOG"

echo "Install Packages"
npm i

echo "Running benchmark"
npm run bench

echo "Removing package-lock.json and node_modules"
[ -d "$NODE_MODULES" ] && rm -rf "$NODE_MODULES"
[ -f "$PACKAGE_LOG" ] && rm "$PACKAGE_LOG"

echo "Done benchmarking"
