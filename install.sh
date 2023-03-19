#!/bin/bash

set -e

ARCH=$(uname -m)
OS=$(uname)

if [ "$OS" != "Linux" ]; then
    echo "Error: This script only supports Linux systems."
    exit 1
fi

case $ARCH in
    aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    arm*) TARGET="arm-unknown-linux-gnueabihf" ;;
    armv7*) TARGET="armv7-unknown-linux-gnueabihf" ;;
    i686) TARGET="i686-unknown-linux-gnu" ;;
    powerpc64le) TARGET="powerpc64le-unknown-linux-gnu" ;;
    s390x) TARGET="s390x-unknown-linux-gnu" ;;
    x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
    *) echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
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
