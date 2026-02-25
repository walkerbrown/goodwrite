#!/usr/bin/env bash
set -euo pipefail

# Upstream GitHub repo
REPO="walkerbrown/goodwrite"

# Installation target for binaries extracted from release archives.
INSTALL_DIR="${GOODWRITE_INSTALL_DIR:-$HOME/.goodwrite/bin}"

# GitHub REST API root used for release/tag discovery.
GITHUB_API_ROOT="https://api.github.com"

usage() {
    cat <<EOF
goodwrite installer

Usage: install.sh [OPTIONS]

Options:
  --version <tag>   Install a specific version (default: latest)
  --uninstall       Remove goodwrite binaries (and ~/.goodwrite when applicable)
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
    # Normalize kernel/architecture names to match release archive naming.
    # Example output: x86_64-apple-darwin, aarch64-unknown-linux-gnu.
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

github_api_get_status() {
    # Perform a GitHub API GET without treating 4xx/5xx as curl failures.
    # The response body is written to the provided file, and the HTTP status
    # code is echoed so the caller can implement explicit error handling.
    local path="$1" output="$2"
    local status

    status="$(curl -sSL -w '%{http_code}' -o "$output" "${GITHUB_API_ROOT}${path}")" \
        || die "failed to contact GitHub API at ${GITHUB_API_ROOT}${path}"

    echo "$status"
}

extract_json_string_field() {
    # Lightweight parser for simple JSON string fields returned by GitHub API.
    # We intentionally keep dependencies minimal and avoid requiring jq.
    local field="$1" file="$2"
    sed -nE "s/.*\"${field}\": *\"([^\"]+)\".*/\1/p" "$file" | head -n 1
}

resolve_version() {
    # Resolve "latest" to a concrete tag so download URLs are deterministic.
    # Strategy:
    # 1) Prefer GitHub Releases latest tag.
    # 2) If no releases exist (404), fall back to newest git tag.
    # 3) Fail with an actionable message if neither exists.
    local requested="$1"
    local version latest_file tags_file latest_status tags_status

    if [ "$requested" != "latest" ]; then
        echo "$requested"
        return 0
    fi

    latest_file="$(mktemp)"
    latest_status="$(github_api_get_status "/repos/${REPO}/releases/latest" "$latest_file")"

    case "$latest_status" in
        200)
            version="$(extract_json_string_field "tag_name" "$latest_file")"
            rm -f "$latest_file"
            [ -n "$version" ] || die "failed to parse latest release tag from ${REPO}"
            echo "$version"
            return 0
            ;;
        404)
            # GitHub returns 404 when a repository exists but has no releases.
            rm -f "$latest_file"
            ;;
        *)
            rm -f "$latest_file"
            die "failed to resolve latest release version from ${REPO} (GitHub API HTTP ${latest_status})"
            ;;
    esac

    tags_file="$(mktemp)"
    tags_status="$(github_api_get_status "/repos/${REPO}/tags?per_page=1" "$tags_file")"
    if [ "$tags_status" != "200" ]; then
        rm -f "$tags_file"
        die "failed to resolve latest tag from ${REPO} (GitHub API HTTP ${tags_status})"
    fi

    version="$(extract_json_string_field "name" "$tags_file")"
    rm -f "$tags_file"

    if [ -z "$version" ]; then
        die "failed to resolve latest release version: ${REPO} has no releases or tags yet"
    fi

    echo "warning: ${REPO} has no releases; using latest tag ${version}" >&2
    echo "$version"
}

verify_checksum() {
    # Verify the downloaded archive digest against the published checksum file.
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
    # Download, verify, and install platform-specific release binaries.
    local version="$1"
    local target

    target="$(detect_platform)"
    version="$(resolve_version "$version")"

    local archive="goodwrite-${target}.tar.gz"
    local base_url="https://github.com/$REPO/releases/download/${version}"
    local tmp
    tmp="$(mktemp -d)"

    # Clean temporary download directory even if installation fails midway.
    trap 'rm -rf "$tmp"' EXIT

    echo "installing goodwrite $version for $target"

    curl -fsSL "${base_url}/${archive}" -o "${tmp}/${archive}" \
        || die "failed to download ${base_url}/${archive}"

    curl -fsSL "${base_url}/${archive}.sha256" -o "${tmp}/${archive}.sha256" \
        || die "failed to download checksum file"

    local expected
    expected="$(awk '{print $1}' "${tmp}/${archive}.sha256")"
    verify_checksum "${tmp}/${archive}" "$expected"

    # Extract binaries into the configured install directory.
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

path_is_within() {
    # Return success when the first path equals the second path or is nested
    # under it. This is used to decide whether uninstall should prune the
    # default ~/.goodwrite directory tree.
    local candidate="${1%/}" parent="${2%/}"
    case "$candidate" in
        "$parent"|"${parent}/"*) return 0 ;;
        *) return 1 ;;
    esac
}

do_uninstall() {
    # Remove previously installed binaries from the configured install location.
    # If install path is inside ~/.goodwrite, remove the whole ~/.goodwrite tree.
    local install_dir="${INSTALL_DIR%/}"
    local goodwrite_root="${HOME}/.goodwrite"

    if path_is_within "$install_dir" "$goodwrite_root"; then
        if [ ! -d "$goodwrite_root" ]; then
            echo "nothing to uninstall (${goodwrite_root} does not exist)"
            return 0
        fi

        rm -rf "$goodwrite_root"
        echo "uninstalled goodwrite from ${goodwrite_root}"
    else
        if [ ! -d "$install_dir" ]; then
            echo "nothing to uninstall (${install_dir} does not exist)"
            return 0
        fi

        local removed=0
        for bin in goodwrite goodwrite-lsp; do
            if [ -f "${install_dir}/${bin}" ]; then
                rm "${install_dir}/${bin}"
                removed=$((removed + 1))
            fi
        done

        if [ -d "$install_dir" ] && [ -z "$(ls -A "$install_dir" 2>/dev/null)" ]; then
            rmdir "$install_dir"
        fi

        if [ "$removed" -eq 0 ]; then
            echo "no goodwrite binaries found in ${install_dir}"
            return 0
        fi

        echo "uninstalled goodwrite from ${install_dir}"
    fi

    case ":${PATH}:" in
        *":${install_dir}:"*)
            echo ""
            echo "you may also want to remove this from your shell profile:"
            echo "  export PATH=\"${install_dir}:\$PATH\""
            ;;
    esac
}

main() {
    # Parse CLI options and dispatch to install or uninstall flow.
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
