#!/bin/sh
# Fledge installer — downloads the latest release binary for your platform.
# Usage: curl -fsSL https://raw.githubusercontent.com/CorvidLabs/fledge/main/install.sh | sh

set -eu

REPO="CorvidLabs/fledge"
INSTALL_DIR="${FLEDGE_INSTALL_DIR:-/usr/local/bin}"

main() {
    need_cmd curl
    need_cmd uname
    need_cmd chmod
    need_cmd mkdir

    local os arch artifact version

    os="$(detect_os)"
    arch="$(detect_arch)"
    artifact="$(artifact_name "$os" "$arch")"
    version="$(latest_version)"

    if [ -z "$version" ]; then
        err "could not determine latest version"
    fi

    say "Installing fledge $version ($os/$arch)..."

    local url="https://github.com/$REPO/releases/download/$version/$artifact"
    local tmpdir
    tmpdir="$(mktemp -d)"
    local tmpfile="$tmpdir/fledge"

    say "Downloading $url"
    curl -fsSL "$url" -o "$tmpfile" || err "download failed — check that $version has a binary for $os/$arch"

    chmod +x "$tmpfile"

    if [ -w "$INSTALL_DIR" ]; then
        mv "$tmpfile" "$INSTALL_DIR/fledge"
    else
        say "Installing to $INSTALL_DIR (requires sudo)"
        sudo mv "$tmpfile" "$INSTALL_DIR/fledge"
    fi

    rm -rf "$tmpdir"

    say "Installed fledge $version to $INSTALL_DIR/fledge"
    say ""
    say "Run 'fledge --help' to get started."
}

detect_os() {
    local uname_s
    uname_s="$(uname -s)"
    case "$uname_s" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) err "unsupported OS: $uname_s" ;;
    esac
}

detect_arch() {
    local uname_m
    uname_m="$(uname -m)"
    case "$uname_m" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        i386|i686)     err "32-bit systems are not supported — fledge requires a 64-bit OS" ;;
        *)             err "unsupported architecture: $uname_m" ;;
    esac
}

artifact_name() {
    local os="$1" arch="$2"
    case "$os" in
        linux)   echo "fledge-linux-$arch" ;;
        macos)   echo "fledge-macos-$arch" ;;
        windows) echo "fledge-windows-$arch.exe" ;;
    esac
}

latest_version() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
        | grep '"tag_name"' \
        | head -1 \
        | sed 's/.*"tag_name": *"//;s/".*//'
}

say() {
    printf "  %s\n" "$@"
}

err() {
    say "Error: $1" >&2
    exit 1
}

need_cmd() {
    if ! command -v "$1" > /dev/null 2>&1; then
        err "need '$1' (not found)"
    fi
}

main
