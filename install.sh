#!/bin/bash

set -e

ARCH=$(uname -m)
OS=$(uname)

case $OS in
    Linux)
        case $ARCH in
            aarch64) TARGET="aarch64-linux-gnu" ;;
            arm*) TARGET="arm-linux-gnueabihf" ;;
            armv7*) TARGET="armv7-linux-gnueabihf" ;;
            i686) TARGET="i686-linux-gnu" ;;
            powerpc64le) TARGET="powerpc64le-linux-gnu" ;;
            s390x) TARGET="s390x-linux-gnu" ;;
            x86_64) TARGET="x86_64-linux-gnu" ;;
            *) echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
        esac
    ;;
    Darwin)
        case $ARCH in
            arm64) TARGET="aarch64-apple-darwin" ;;
            x86_64) TARGET="x86_64-apple-darwin" ;;
            *) echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
        esac
    ;;
    *)
        echo "Error: This script only supports Linux and macOS systems."
        exit 1
    ;;
esac

echo "Detected architecture: $TARGET"

REPO="https://github.com/jkfran/killport"
LATEST_RELEASE_URL="$REPO/releases/latest/download/killport-$TARGET.tar.gz"
INSTALL_DIR="${HOME}/.local/bin"

echo "Downloading killport..."
curl -sL "$LATEST_RELEASE_URL" -o "/tmp/killport-$TARGET.tar.gz"

echo "Extracting killport..."
mkdir -p "${INSTALL_DIR}"
tar -xzf "/tmp/killport-$TARGET.tar.gz" -C "${INSTALL_DIR}"

echo "killport has been installed to ${INSTALL_DIR}/killport"
echo "Please ensure ${INSTALL_DIR} is in your PATH."
