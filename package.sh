#!/usr/bin/env bash
# Build release binary and pack for distribution (Linux).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

NAME="geo-ring-viewer"
VERSION="$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')"
ARCH="$(uname -m)"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
DIST_NAME="${NAME}-${VERSION}-${OS}-${ARCH}"
DIST_DIR="$ROOT/dist/$DIST_NAME"
ARCHIVE="$ROOT/dist/${DIST_NAME}.tar.gz"

echo "==> Building release..."
cargo build --release

BIN="$ROOT/target/release/$NAME"
if [[ ! -f "$BIN" ]]; then
  echo "Error: binary not found at $BIN" >&2
  exit 1
fi

echo "==> Packing into $DIST_DIR"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/examples"

cp "$BIN" "$DIST_DIR/"
cp -r examples/*.txt "$DIST_DIR/examples/" 2>/dev/null || true
cp README.md "$DIST_DIR/"
cp DISTRIBUTE.md "$DIST_DIR/" 2>/dev/null || true

chmod +x "$DIST_DIR/$NAME"

echo "==> Creating archive $ARCHIVE"
mkdir -p "$ROOT/dist"
tar -czf "$ARCHIVE" -C "$ROOT/dist" "$DIST_NAME"

echo ""
echo "Done."
echo "  Folder:  $DIST_DIR"
echo "  Archive: $ARCHIVE"
echo ""
echo "Send your colleague the .tar.gz file. They extract and run:"
echo "  tar -xzf ${DIST_NAME}.tar.gz"
echo "  cd $DIST_NAME"
echo "  ./$NAME"
