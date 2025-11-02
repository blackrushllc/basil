#!/usr/bin/env bash
# Release Basil to multiple Ubuntu series via Launchpad PPA
# Usage:
#   ./scripts/release-multi.sh <GPG_KEY_ID> [series...]
#   ./scripts/release-multi.sh <GPG_KEY_ID> --new-upstream 1.2.0 [series...]
# Notes:
# - Run from anywhere; the script will cd to the repo root.
# - Requires: devscripts (dch, debuild), dput, git

set -euo pipefail

# --- find repo root ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# --- args ---
KEYID="${1:-${DEBSIGN_KEYID:-}}"
if [[ -z "$KEYID" ]]; then
  echo "❌ Please pass your GPG key ID. Example:"
  echo "   ./scripts/release-multi.sh ABCDEF1234567890 [series...]"
  exit 1
fi
shift || true

NEW_UPSTREAM=""
if [[ "${1:-}" == "--new-upstream" ]]; then
  shift
  NEW_UPSTREAM="${1:-}"
  if [[ -z "$NEW_UPSTREAM" ]]; then
    echo "❌ --new-upstream requires a version, e.g. 1.2.0"
    exit 1
  fi
  shift || true
fi

SERIES_LIST=("$@")
if [[ ${#SERIES_LIST[@]} -eq 0 ]]; then
  SERIES_LIST=(noble jammy focal)
fi

# --- helpers ---
changelog_version() { dpkg-parsechangelog -S Version | tr -d '[:space:]'; }
upstream_from_version() { local v="$1"; echo "${v%%-*}"; }
extract_deb_rev() {
  local v="$1"; echo "$v" | sed -n 's/.*-0ubuntu\([0-9]\+\).*/\1/p' | { read -r n || true; [[ -n "${n:-}" ]] && echo "$n" || echo 0; }
}
upload_changes() {
  local changes="$1"
  local parent="$REPO_ROOT/.."
  local base; base="$(basename "$changes")"
  rm -f "$parent/${base}.upload" || true
  (cd "$parent" && dput ppa:blackrush/basil "$base")
}

# --- sanity: offline cargo bits present ---
if [[ ! -f debian/cargo-vendor.tar.xz ]]; then
  echo "❌ debian/cargo-vendor.tar.xz is missing."
  exit 1
fi
if [[ ! -f debian/cargo-checksum.json ]]; then
  echo "❌ debian/cargo-checksum.json missing."
  exit 1
fi
if [[ ! -f debian/source/include-binaries ]] || ! grep -q 'debian/cargo-vendor.tar.xz' debian/source/include-binaries; then
  echo "⚠️  Adding debian/cargo-vendor.tar.xz to debian/source/include-binaries"
  mkdir -p debian/source
  printf 'debian/cargo-vendor.tar.xz\n' > debian/source/include-binaries
fi

# --- version prep ---
CUR_VER="$(changelog_version)"                  # e.g. 1.1.0-0ubuntuN...
CUR_UPSTREAM="$(upstream_from_version "$CUR_VER")"
CUR_DEB_REV="$(extract_deb_rev "$CUR_VER")"
DEB_BASE="$CUR_DEB_REV"

# If starting a new upstream cycle, switch upstream baseline now.
if [[ -n "$NEW_UPSTREAM" ]]; then
  CUR_UPSTREAM="$NEW_UPSTREAM"
  # Require canonical orig in parent, fail if missing
  PARENT_ORIG="../basil_${CUR_UPSTREAM}.orig.tar.gz"
  if [[ ! -f "$PARENT_ORIG" ]]; then
    echo "❌ Missing ../basil_${CUR_UPSTREAM}.orig.tar.gz"
    echo "   Create it from your release tag, e.g.:"
    echo "   (from repo root)  cd .. && git -C basil archive --format=tar --prefix=\"basil-${CUR_UPSTREAM}/\" v${CUR_UPSTREAM} | gzip -n > basil_${CUR_UPSTREAM}.orig.tar.gz"
    exit 1
  fi
  echo "ℹ️  New upstream cycle: $CUR_UPSTREAM  (will upload orig once with -sa)"
  # Restart Debian rev sequence for the new cycle
  DEB_BASE=0
else
  echo "ℹ️  Continuing on upstream: $CUR_UPSTREAM  (no orig upload; using -sd)"
fi

# --- main loop ---
FIRST_SERIES=true
for SERIES in "${SERIES_LIST[@]}"; do
  echo
  echo "🌀 Building for ${SERIES}..."

  DEB_BASE=$((DEB_BASE + 1))
  NEW_VER="${CUR_UPSTREAM}-0ubuntu${DEB_BASE}~${SERIES}1"
  CHANGELOG_MSG="Build for Ubuntu ${SERIES}"

  if $FIRST_SERIES && [[ -n "$NEW_UPSTREAM" ]]; then
    CHANGELOG_MSG="${CHANGELOG_MSG} (uploads orig)"
  else
    CHANGELOG_MSG="${CHANGELOG_MSG} (no orig upload)"
  fi

  dch --newversion "$NEW_VER" -D "$SERIES" "$CHANGELOG_MSG."

  if $FIRST_SERIES && [[ -n "$NEW_UPSTREAM" ]]; then
    debuild -S -sa -k"$KEYID"   # upload orig ONCE for new upstream
    FIRST_SERIES=false
  else
    debuild -S -sd -k"$KEYID"   # subsequent uploads: no orig
  fi

  CHANGES="../basil_${NEW_VER}_source.changes"
  if [[ ! -f "$CHANGES" ]]; then
    echo "❌ Expected changes file not found: $CHANGES"
    exit 1
  fi

  echo "🚀 Uploading $CHANGES ..."
  upload_changes "$CHANGES"
  echo "✅ Uploaded for ${SERIES}: basil_${NEW_VER}_source.changes"
  echo "------------------------------------------"
done

echo "🎉 All uploads queued to PPA. Watch Launchpad for build/publish status."
