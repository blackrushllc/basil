#!/usr/bin/env bash
set -euo pipefail

if ! command -v debuild >/dev/null 2>&1; then
  echo "Please: sudo apt update && sudo apt install devscripts debhelper dpkg-dev dput gnupg build-essential fakeroot lintian"
  exit 1
fi

PPA="${1:-ppa:blackrush/basil}"  # override with ./scripts/debian/release.sh ppa:YOURNAME/basil
DIST="${2:-noble}"               # target series; adjust or omit to let Launchpad map automatically

# Bump changelog interactively if needed:
# dch -i  (or: dch -v 1.0.1-0ubuntu1 "New upstream release.")

# Build signed source package (.changes ends with _source.changes)
debuild -S -sa

# Find the newest source changes and upload to the PPA
CHANGES=$(ls -1t ../basil_*_source.changes | head -n1)
echo "Uploading $CHANGES to $PPA ..."
dput "$PPA" "$CHANGES"

echo "Done. Check Launchpad build status in your PPA."
