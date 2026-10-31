#!/usr/bin/env bash
set -euo pipefail

# Fixing a GPG key database lock

# `npm run fix:gpg` is a local recovery script for the package author's Fedora
# Workstation. It fixes a GPG key database lock by restarting the GPG agent and
# removing the lock file. Do not run this command on other systems.

gpgconf --kill gpg-agent

sleep 2

rm ~/.gnupg/public-keys.d/pubring.db.lock

sleep 2

gpgconf --launch gpg-agent
