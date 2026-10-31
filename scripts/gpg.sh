#!/usr/bin/env bash
set -euo pipefail

# This file is only run to fix gpg key db lock
# for package author local host system (Fedora Workstation)

# Do not run npm run fix:gpg in other systems.

gpgconf --kill gpg-agent

sleep 2

rm ~/.gnupg/public-keys.d/pubring.db.lock

sleep 2

gpgconf --launch gpg-agent
