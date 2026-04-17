#!/usr/bin/env bash
set -e

# Graphyn Installation Script

GITHUB_REPO="JeelGajera/graphyn"
INSTALL_DIR="${HOME}/.local/bin"

if [ "$EUID" -eq 0 ]; then
    INSTALL_DIR="/usr/local/bin"
fi

echo "=========================================="
echo "⚡ Installing Graphyn Code Intelligence Engine"
echo "=========================================="

# 1. Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     PLATFORM="unknown-linux-gnu";;
    Darwin*)    PLATFORM="apple-darwin";;
    *)          echo "Error: Unsupported OS '${OS}'"; exit 1;;
esac

# 2. Detect Architecture
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64*)    ARCH="x86_64";;
    arm64*|aarch64*) ARCH="aarch64";;
    *)          echo "Error: Unsupported architecture '${ARCH}'"; exit 1;;
esac

TARGET="${ARCH}-${PLATFORM}"
ASSET_NAME="graphyn-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/${ASSET_NAME}"

echo "Detected Platform: ${OS} (${ARCH})"
echo "Target: ${TARGET}"

# 3. Create temp directory
TMP_DIR=$(mktemp -d -t graphyn-install-XXXXXXXXXX)
trap 'rm -rf "${TMP_DIR}"' EXIT

cd "${TMP_DIR}"

# 4. Download
echo "Downloading latest release..."
if command -v curl &> /dev/null; then
    curl -fsSLO "${DOWNLOAD_URL}"
elif command -v wget &> /dev/null; then
    wget -qO "${ASSET_NAME}" "${DOWNLOAD_URL}"
else
    echo "Error: Neither curl nor wget is installed."
    exit 1
fi

# 5. Extract
echo "Extracting binary..."
tar -xzf "${ASSET_NAME}"
chmod +x graphyn

# 6. Install
echo "Installing to ${INSTALL_DIR}..."
mkdir -p "${INSTALL_DIR}"

if command -v graphyn &> /dev/null; then
    OLD_VERSION=$(graphyn --version 2>/dev/null || echo "unknown")
    echo "Updating existing installation (${OLD_VERSION})..."
fi

mv graphyn "${INSTALL_DIR}/graphyn"

# 7. Check PATH
NEW_VERSION=$("${INSTALL_DIR}/graphyn" --version)

echo ""
echo "✅ Successfully installed ${NEW_VERSION}!"
echo ""

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "⚠️  WARNING: ${INSTALL_DIR} is not in your PATH."
    echo "   Please add the following to your .bashrc, .zshrc, or equivalent:"
    echo ""
    echo "   export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
else
    echo "Graphyn is ready to use! Run 'graphyn --help' to get started."
fi
echo "=========================================="

