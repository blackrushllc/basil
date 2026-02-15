#!/usr/bin/env bash
set -euo pipefail

# gen-latest-json.sh
# Generates downloads/latest.json from the highest semver-ish directory under downloads/.
#
# Expected tree:
# downloads/
#   1.0.0/
#     linux/   (basilc, bcc, variants...)
#     mac/     (...)
#     windows/ (... .exe/.msi ...)
#
# Output:
# downloads/latest.json
#
# Usage:
#   cd /path/to/site-root/downloads
#   ./gen-latest-json.sh
#
# Optional env:
#   BASE_URL="https://basrun.com/downloads"   (default shown)
#   OUT_FILE="latest.json"                   (default shown)

BASE_URL="${BASE_URL:-https://basrun.com/downloads}"
OUT_FILE="${OUT_FILE:-latest.json}"

# Locate downloads root (directory containing this script)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DL_ROOT="$SCRIPT_DIR"

# sha256 helper (macOS uses shasum; Linux usually has sha256sum)
sha256_file() {
  local f="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$f" | awk '{print $1}'
  else
    echo "ERROR: need sha256sum or shasum" >&2
    exit 1
  fi
}

# size helper (portable-ish)
file_size() {
  local f="$1"
  if stat -c '%s' "$f" >/dev/null 2>&1; then
    stat -c '%s' "$f"
  else
    stat -f '%z' "$f"
  fi
}

# mtime helper (epoch seconds, portable-ish)
file_mtime() {
  local f="$1"
  if stat -c '%Y' "$f" >/dev/null 2>&1; then
    stat -c '%Y' "$f"
  else
    stat -f '%m' "$f"
  fi
}

# JSON string escape (enough for filenames/paths)
json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

# Find latest version dir (semver-ish)
latest_version="$(
  find "$DL_ROOT" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null \
    | sort -V | tail -n 1
)"

# macOS/BSD find doesn't support -printf
if [[ -z "${latest_version:-}" ]]; then
  latest_version="$(
    find "$DL_ROOT" -mindepth 1 -maxdepth 1 -type d \
      | sed 's#.*/##' \
      | sort -V | tail -n 1
  )"
fi

if [[ -z "${latest_version:-}" ]]; then
  echo "ERROR: No version directories found under: $DL_ROOT" >&2
  exit 1
fi

VER_DIR="$DL_ROOT/$latest_version"

# Sanity check expected OS dirs (some may be missing; that's ok)
OS_DIRS=("linux" "mac" "windows")

tmp_out="$(mktemp)"
trap 'rm -f "$tmp_out"' EXIT

{
  echo "{"
  echo "  \"version\": \"$(json_escape "$latest_version")\","
  echo "  \"base_url\": \"$(json_escape "$BASE_URL")/$(json_escape "$latest_version")\","
  echo "  \"generated_at_utc\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\","
  echo "  \"artifacts\": {"

  first_artifact=1

  # We'll build a map like:
  # artifacts: {
  #   "basilc": { "linux": {...}, "mac": {...}, "windows": {...} },
  #   "bcc":    { ... },
  #   "Basil.msi": { "windows": {...} }, etc...
  #
  # Key is the filename (without path). For windows, includes .exe/.msi if present.

  declare -A seen_keys=()

  for os in "${OS_DIRS[@]}"; do
    dir="$VER_DIR/$os"
    [[ -d "$dir" ]] || continue

    # list regular files only
    while IFS= read -r -d '' f; do
      fname="$(basename "$f")"
      key="$fname"  # keep extension for windows/MSI, also fine for linux/mac
      # Remember key exists (we’ll emit per key across OSes)
      seen_keys["$key"]=1
    done < <(find "$dir" -maxdepth 1 -type f -print0)
  done

  # Sort keys for stable output
  mapfile -t keys_sorted < <(printf "%s\n" "${!seen_keys[@]}" | sort)

  for key in "${keys_sorted[@]}"; do
    [[ $first_artifact -eq 1 ]] || echo "    ,"
    first_artifact=0

    echo "    \"$(json_escape "$key")\": {"

    first_os=1
    for os in "${OS_DIRS[@]}"; do
      fpath="$VER_DIR/$os/$key"
      [[ -f "$fpath" ]] || continue

      sha="$(sha256_file "$fpath")"
      size="$(file_size "$fpath")"
      mtime="$(file_mtime "$fpath")"

      [[ $first_os -eq 1 ]] || echo "      ,"
      first_os=0

      echo "      \"$(json_escape "$os")\": {"
      echo "        \"file\": \"$(json_escape "$os")/$(json_escape "$key")\","
      echo "        \"sha256\": \"$(json_escape "$sha")\","
      echo "        \"size\": $size,"
      echo "        \"mtime\": $mtime"
      echo "      }"
    done

    echo "    }"
  done

  echo "  }"
  echo "}"
} > "$tmp_out"

# Write output atomically
out_path="$DL_ROOT/$OUT_FILE"
mv "$tmp_out" "$out_path"

echo "Wrote: $out_path"
echo "Latest version: $latest_version"
