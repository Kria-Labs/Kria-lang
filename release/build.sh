#!/usr/bin/env bash
# Kria-lang Build Script (Linux/macOS)
# Usage:
#   ./build.sh              - Build release binary
#   ./build.sh install      - Build and install to ~/.kria/bin
#   ./build.sh clean        - Clean build artifacts
#   ./build.sh package      - Build and create tar.gz in release/
#   ./build.sh help         - Show this help

set -e

PROJECT_NAME="kria"
BINARY_NAME="kria"
INSTALL_DIR="$HOME/.kria/bin"

# Detect arch
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) ARCH="$ARCH" ;;
esac

# Detect OS
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"

get_version() {
    grep 'version = "' Cargo.toml | head -1 | sed 's/.*version = "\([^"]*\)".*/\1/'
}

case "${1:-}" in
    help|--help|-h)
        echo "Kria-lang Build Script"
        echo ""
        echo "Usage:"
        echo "  ./build.sh              Build release binary"
        echo "  ./build.sh install      Build and install to $INSTALL_DIR"
        echo "  ./build.sh clean        Clean build artifacts"
        echo "  ./build.sh package      Build and create tar.gz in release/"
        echo "  ./build.sh help         Show this help"
        exit 0
        ;;
    clean)
        echo "[*] Cleaning build artifacts..."
        cargo clean
        rm -rf release/*
        echo "[+] Clean done."
        exit 0
        ;;
esac

echo "[*] Building $PROJECT_NAME (release)..."
cargo build --release

BINARY_PATH="target/release/$BINARY_NAME"
if [ ! -f "$BINARY_PATH" ]; then
    echo "[-] Binary not found at $BINARY_PATH"
    exit 1
fi

echo "[+] Build successful: $BINARY_PATH"

case "${1:-}" in
    install)
        echo "[*] Installing to $INSTALL_DIR..."
        mkdir -p "$INSTALL_DIR"
        cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
        chmod +x "$INSTALL_DIR/$BINARY_NAME"
        echo "[+] Installed: $INSTALL_DIR/$BINARY_NAME"

        # Check PATH
        case ":$PATH:" in
            *":$INSTALL_DIR:"*)
                ;;
            *)
                echo ""
                echo "[!] Add $INSTALL_DIR to your PATH:"
                echo "    echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.bashrc"
                echo "    source ~/.bashrc"
                ;;
        esac

        echo ""
        echo "[*] Verifying installation..."
        "$INSTALL_DIR/$BINARY_NAME" 2>/dev/null && true
        echo "[+] Binary is executable."
        ;;

    package)
        VERSION="$(get_version)"
        PKG_NAME="${PROJECT_NAME}-${VERSION}-${OS}-${ARCH}.tar.gz"
        PKG_PATH="release/$PKG_NAME"

        mkdir -p release
        echo "[*] Packaging to $PKG_PATH..."

        # Stage files
        STAGING=$(mktemp -d)
        cp "$BINARY_PATH" "$STAGING/$BINARY_NAME"
        cp README.md "$STAGING/"
        cp LICENSE "$STAGING/"
        cp test.krx "$STAGING/" 2>/dev/null || true

        tar -czf "$PKG_PATH" -C "$STAGING" .
        rm -rf "$STAGING"

        echo "[+] Package created: $PKG_PATH"
        ;;
esac

echo "[+] Done."
