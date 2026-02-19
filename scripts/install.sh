#!/usr/bin/env bash
set -euo pipefail

REPO="walkerbrown/goodwrite"
INSTALL_DIR="${GOODWRITE_INSTALL_DIR:-$HOME/.goodwrite/bin}"

usage() {
    cat <<EOF
goodwrite installer

Usage: install.sh [OPTIONS]

Options:
  --version <tag>   Install a specific version (default: latest)
  --uninstall       Remove goodwrite binaries
  --help            Show this help message

Environment:
  GOODWRITE_INSTALL_DIR   Override install directory (default: ~/.goodwrite/bin)

Examples:
  curl -fsSL https://raw.githubusercontent.com/$REPO/main/scripts/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/$REPO/main/scripts/install.sh | bash -s -- --version v0.1.0
EOF
}

die() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        *)      die "unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64)          arch="x86_64" ;;
        aarch64|arm64)   arch="aarch64" ;;
        *)               die "unsupported architecture: $arch" ;;
    esac

    echo "${arch}-${os}"
}

resolve_version() {
    local version="$1"
    if [ "$version" = "latest" ]; then
        version="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
            | grep '"tag_name"' \
            | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
        if [ -z "$version" ]; then
            die "failed to resolve latest release version"
        fi
    fi
    echo "$version"
}

verify_checksum() {
    local file="$1" expected="$2"

    local actual
    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$file" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        actual="$(shasum -a 256 "$file" | awk '{print $1}')"
    else
        echo "warning: no sha256 tool found, skipping checksum verification" >&2
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        die "checksum mismatch: expected $expected, got $actual"
    fi
}

do_install() {
    local version="$1"
    local target

    target="$(detect_platform)"
    version="$(resolve_version "$version")"

    local archive="goodwrite-${target}.tar.gz"
    local base_url="https://github.com/$REPO/releases/download/${version}"
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT

    echo "installing goodwrite $version for $target"

    curl -fsSL "${base_url}/${archive}" -o "${tmp}/${archive}" \
        || die "failed to download ${base_url}/${archive}"

    curl -fsSL "${base_url}/${archive}.sha256" -o "${tmp}/${archive}.sha256" \
        || die "failed to download checksum file"

    local expected
    expected="$(awk '{print $1}' "${tmp}/${archive}.sha256")"
    verify_checksum "${tmp}/${archive}" "$expected"

    mkdir -p "$INSTALL_DIR"
    tar -xzf "${tmp}/${archive}" -C "$INSTALL_DIR"
    chmod +x "${INSTALL_DIR}/goodwrite" "${INSTALL_DIR}/goodwrite-lsp"

    echo "installed goodwrite to ${INSTALL_DIR}"

    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo ""
            echo "add goodwrite to your PATH:"
            echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
            echo ""
            echo "to make it permanent, add the line above to your shell profile"
            echo "(~/.bashrc, ~/.zshrc, or ~/.profile)"
            ;;
    esac
}

do_uninstall() {
    if [ ! -d "$INSTALL_DIR" ]; then
        echo "nothing to uninstall (${INSTALL_DIR} does not exist)"
        return 0
    fi

    local removed=0
    for bin in goodwrite goodwrite-lsp; do
        if [ -f "${INSTALL_DIR}/${bin}" ]; then
            rm "${INSTALL_DIR}/${bin}"
            removed=$((removed + 1))
        fi
    done

    if [ "$removed" -eq 0 ]; then
        echo "no goodwrite binaries found in ${INSTALL_DIR}"
        return 0
    fi

    # Remove directory if empty
    if [ -d "$INSTALL_DIR" ] && [ -z "$(ls -A "$INSTALL_DIR" 2>/dev/null)" ]; then
        rmdir "$INSTALL_DIR"
    fi

    echo "uninstalled goodwrite from ${INSTALL_DIR}"

    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*)
            echo ""
            echo "you may also want to remove this from your shell profile:"
            echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
            ;;
    esac
}

main() {
    local version="latest"
    local uninstall=false

    while [ $# -gt 0 ]; do
        case "$1" in
            --version)
                shift
                [ $# -gt 0 ] || die "--version requires a value"
                version="$1"
                ;;
            --uninstall)
                uninstall=true
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                die "unknown option: $1 (see --help)"
                ;;
        esac
        shift
    done

    if [ "$uninstall" = true ]; then
        do_uninstall
    else
        do_install "$version"
    fi
}

main "$@"
