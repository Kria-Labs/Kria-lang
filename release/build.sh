#!/usr/bin/env bash
# Kria build & install (Linux/macOS)
#
# Run from anywhere:
#   ./release/build.sh              # build only
#   ./release/build.sh install      # build + install to ~/.kria/bin
#   ./release/build.sh package      # build + tar.gz in release/
#   ./release/build.sh clean        # cargo clean + remove release artifacts
#   ./release/build.sh help

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

PROJECT_NAME="kria"
BINARY_NAME="kria"
INSTALL_DIR="${HOME}/.kria/bin"
# Always use the project target dir so install paths are predictable
export CARGO_TARGET_DIR="${PROJECT_ROOT}/target"
BINARY_PATH="${CARGO_TARGET_DIR}/release/${BINARY_NAME}"

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
esac

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"

get_version() {
    grep 'version = "' "${PROJECT_ROOT}/Cargo.toml" | head -1 | sed 's/.*version = "\([^"]*\)".*/\1/'
}

usage() {
    echo "Kria build script (project root: ${PROJECT_ROOT})"
    echo ""
    echo "Usage:"
    echo "  ${SCRIPT_DIR}/build.sh              Build release binary"
    echo "  ${SCRIPT_DIR}/build.sh install      Build, install to ${INSTALL_DIR}, update shell PATH"
    echo "  ${SCRIPT_DIR}/build.sh install --no-path   Install without editing shell rc files"
    echo "  ${SCRIPT_DIR}/build.sh package      Build and create tar.gz in release/"
    echo "  ${SCRIPT_DIR}/build.sh clean        Clean build artifacts"
    echo "  ${SCRIPT_DIR}/build.sh help         Show this help"
}

path_on_path() {
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) return 0 ;;
        *) return 1 ;;
    esac
}

# Append PATH export to shell rc files if missing
setup_shell_path() {
    local line="export PATH=\"\${PATH}:${INSTALL_DIR}\""
    local updated=0

    for rc in "${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile"; do
        [ -f "${rc}" ] || continue
        if grep -Fq "${INSTALL_DIR}" "${rc}" 2>/dev/null; then
            continue
        fi
        {
            echo ""
            echo "# Kria (${PROJECT_NAME})"
            echo "${line}"
        } >> "${rc}"
        echo "[+] Added PATH to ${rc}"
        updated=1
    done

    if [ -d "${HOME}/.config/fish/conf.d" ]; then
        local fish_conf="${HOME}/.config/fish/conf.d/kria.fish"
        if [ ! -f "${fish_conf}" ]; then
            echo "fish_add_path ${INSTALL_DIR}" > "${fish_conf}"
            echo "[+] Created ${fish_conf}"
            updated=1
        fi
    fi

    if [ "${updated}" -eq 0 ] && path_on_path; then
        echo "[+] ${INSTALL_DIR} is already on PATH."
    elif [ "${updated}" -eq 0 ]; then
        echo "[!] Could not find ~/.bashrc or ~/.zshrc to update."
        echo "    Run manually: export PATH=\"\${PATH}:${INSTALL_DIR}\""
    else
        echo ""
        echo "[!] Open a new terminal, or run:"
        echo "    source ~/.bashrc    # bash"
        echo "    source ~/.zshrc     # zsh"
        echo ""
        echo "    Then test: kria --help"
    fi
}

do_build() {
    if ! command -v cargo >/dev/null 2>&1; then
        echo "[-] Rust/cargo not found. Install from https://rustup.rs/" >&2
        exit 1
    fi

    echo "[*] Building ${PROJECT_NAME} (release)..."
    echo "[*] Target directory: ${CARGO_TARGET_DIR}"
    cargo build --release

    if [ ! -f "${BINARY_PATH}" ]; then
        echo "[-] Binary not found at ${BINARY_PATH}" >&2
        echo "[-] Try: cd ${PROJECT_ROOT} && cargo build --release" >&2
        exit 1
    fi

    echo "[+] Build successful: ${BINARY_PATH}"
}

case "${1:-}" in
    help|--help|-h)
        usage
        exit 0
        ;;
    clean)
        echo "[*] Cleaning..."
        cargo clean
        # Remove packaged artifacts only; keep build.sh and installer scripts
        find "${SCRIPT_DIR}" -mindepth 1 -maxdepth 1 \
            \( -name '*.tar.gz' -o -name '*.zip' \) -delete 2>/dev/null || true
        echo "[+] Clean done."
        exit 0
        ;;
esac

do_build

case "${1:-}" in
    install)
        SETUP_PATH=1
        if [ "${2:-}" = "--no-path" ]; then
            SETUP_PATH=0
        fi

        echo "[*] Installing to ${INSTALL_DIR}..."
        mkdir -p "${INSTALL_DIR}"
        cp "${BINARY_PATH}" "${INSTALL_DIR}/${BINARY_NAME}"
        chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
        echo "[+] Installed: ${INSTALL_DIR}/${BINARY_NAME}"

        if [ "${SETUP_PATH}" -eq 1 ]; then
            if path_on_path; then
                echo "[+] ${INSTALL_DIR} is already on PATH in this shell."
            else
                echo ""
                echo "[*] Updating shell config so \`kria\` works in new terminals..."
                setup_shell_path
            fi
        else
            echo ""
            echo "[!] PATH not modified. Use full path: ${INSTALL_DIR}/kria"
        fi

        echo ""
        echo "[*] Verifying..."
        if "${INSTALL_DIR}/${BINARY_NAME}" --help >/dev/null 2>&1; then
            echo "[+] \`kria --help\` works."
        else
            echo "[-] Installed binary failed --help check." >&2
            exit 1
        fi
        echo ""
        echo "[+] Run: kria          (REPL)"
        echo "[+]      kria file.krx"
        ;;

    package)
        VERSION="$(get_version)"
        PKG_NAME="${PROJECT_NAME}-${VERSION}-${OS}-${ARCH}.tar.gz"
        PKG_PATH="${SCRIPT_DIR}/${PKG_NAME}"

        mkdir -p "${SCRIPT_DIR}"
        echo "[*] Packaging ${PKG_PATH}..."

        STAGING="$(mktemp -d)"
        trap 'rm -rf "${STAGING}"' EXIT
        cp "${BINARY_PATH}" "${STAGING}/${BINARY_NAME}"
        cp README.md LICENSE "${STAGING}/"
        cp test.krx "${STAGING}/" 2>/dev/null || true

        tar -czf "${PKG_PATH}" -C "${STAGING}" .
        echo "[+] Package created: ${PKG_PATH}"
        ;;

    "")
        echo ""
        echo "[*] Binary ready. To install system-wide:"
        echo "    ${SCRIPT_DIR}/build.sh install"
        echo ""
        echo "[*] Or run without installing:"
        echo "    ${BINARY_PATH} test.krx"
        ;;

    *)
        echo "[-] Unknown command: ${1}" >&2
        usage >&2
        exit 1
        ;;
esac

echo "[+] Done."
