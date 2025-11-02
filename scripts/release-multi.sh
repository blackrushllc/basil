#!/usr/bin/env bash
# Release Basil to multiple Ubuntu series via Launchpad PPA
# Usage: ./scripts/release-multi.sh <DEBSIGN_KEYID>

set -euo pipefail
KEYID="${1:-${DEBSIGN_KEYID:-}}"
if [[ -z "$KEYID" ]]; then
  echo "❌ Please pass your GPG key ID, e.g.: ./scripts/release-multi.sh ABCDEF1234567890"
  exit 1
fi

# Series you want to target
SERIES_LIST=("noble" "jammy" "focal")

VERSION_BASE="1.1.0"
cd "$(dirname "$0")/.."

for SERIES in "${SERIES_LIST[@]}"; do
  echo "🌀 Building for $SERIES..."

  # Increment Debian rev automatically
  dch -i -D "$SERIES" "Build for Ubuntu $SERIES."

  # Build source-only package
  debuild -S -sa -k"$KEYID"

  # Find the newest .changes file
  CHANGES_FILE=$(ls -t ../basil_${VERSION_BASE}-0ubuntu*_source.changes | head -n1)

  echo "🚀 Uploading $CHANGES_FILE to PPA..."
  (cd .. && dput ppa:blackrush/basil "$(basename "$CHANGES_FILE")")

  echo "✅ Done: $SERIES"
  echo "------------------------------------------"
done
