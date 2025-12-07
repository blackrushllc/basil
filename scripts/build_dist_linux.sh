#!/usr/bin/env bash
# Build various Basil release binaries for Linux and copy them into dist/linux
# Variants follow the requested naming scheme:
# - No --features                     -> basilc-naked, bcc-naked
# - --features obj-all                -> basilc, bcc
# - --features obj-bmx                -> basilc-bmx, bcc-bmx
# - --features "obj-json obj-daw obj-term obj-sqlite" -> basilc-daw, bcc-daw
# - --features "obj-ai obj-csv obj-curl obj-json obj-zip obj-sqlite obj-aws obj-sql obj-orm obj-net" -> basilc-web, bcc-web


#cargo build -p basilc --release --features obj-all
#install -m 0755 target/release/basilc /usr/lib/cgi-bin/basil.cgi

set -euo pipefail

# Resolve repository root and move there (script may be invoked from anywhere)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "$REPO_ROOT"

DIST_DIR="$REPO_ROOT/dist/linux"
mkdir -p "$DIST_DIR"

BASILC_BIN="$REPO_ROOT/target/release/basilc"
BCC_BIN="$REPO_ROOT/target/release/bcc"

build_variant() {
  local label="$1"; shift
  local features="${1:-}"

  echo -e "\n=== Building variant: ${label} ==="
  if [[ -z "$features" ]]; then
    echo "cargo build -p basilc --release"
    cargo build -p basilc --release
  else
    echo "cargo build -p basilc --release --features \"$features\""
    cargo build -p basilc --release --features "$features"
  fi

  # Build bcc (no features to pass for this crate)
  echo "cargo build -p bcc --release"
  cargo build -p bcc --release

  # Verify binaries exist
  [[ -x "$BASILC_BIN" ]] || { echo "Error: basilc binary not found at $BASILC_BIN" >&2; exit 1; }
  [[ -x "$BCC_BIN"    ]] || { echo "Error: bcc binary not found at $BCC_BIN" >&2; exit 1; }

  local basilc_out
  local bcc_out
  case "$label" in
    naked)
      basilc_out="basilc-naked"
      bcc_out="bcc-naked"
      ;;
    all)
      basilc_out="basilc"
      bcc_out="bcc"
      ;;
    bmx)
      basilc_out="basilc-bmx"
      bcc_out="bcc-bmx"
      ;;
    daw)
      basilc_out="basilc-daw"
      bcc_out="bcc-daw"
      ;;
    web)
      basilc_out="basilc-web"
      bcc_out="bcc-web"
      ;;
    *)
      echo "Unknown label: $label" >&2
      exit 1
      ;;
  esac

  cp -f "$BASILC_BIN" "$DIST_DIR/$basilc_out"
  cp -f "$BCC_BIN" "$DIST_DIR/$bcc_out"
  chmod +x "$DIST_DIR/$basilc_out" "$DIST_DIR/$bcc_out"
  echo "Copied to $DIST_DIR/$basilc_out and $DIST_DIR/$bcc_out"
}

# Variants
build_variant naked ""
build_variant bmx   "obj-bmx"
build_variant daw   "obj-json obj-daw obj-term obj-sqlite"
build_variant web   "obj-ai obj-csv obj-curl obj-json obj-zip obj-sqlite obj-aws obj-sql obj-orm obj-net"
build_variant all   "obj-all"

install -m 0755 target/release/basilc /usr/lib/cgi-bin/basil.cgi


echo -e "\nAll Linux builds completed. Output directory: $DIST_DIR"
