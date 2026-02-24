#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-full}"
shift || true

CHANNEL="stable"
OFFLINE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --channel)
      CHANNEL="${2:-stable}"
      shift 2
      ;;
    --offline)
      OFFLINE=1
      shift
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

run_core() {
  echo "--> Building workspace..."
  cargo build -q --workspace --locked
  
  echo "--> Running tests..."
  # Explicitly run lib, tests, and bins to suppress empty Doc-tests output
  cargo test -q --workspace --locked --lib --tests --bins

  if [[ "$CHANNEL" == "stable" ]]; then
    echo "--> Checking formatting..."
    cargo fmt --all -- --check
    
    echo "--> Running clippy..."
    cargo clippy -q --workspace --all-targets -- -D warnings
  fi
}

run_accountability() {
  if git ls-files | grep -E 'ASD-STE100.*\.pdf$' >/dev/null; then
    echo "tracked ASD PDF detected" >&2
    exit 1
  fi

  python3 scripts/validate_rule_index.py
  cargo test -p goodwrite-core --test rule_index_test

  local linkage_log
  linkage_log="$(mktemp -t goodwrite-rule-linkage.XXXXXX.log)"
  cargo test -p goodwrite-cli --test rule_accountability -- --nocapture | tee "$linkage_log"

  python3 scripts/update_rule_coverage_badge.py --from-log "$linkage_log" --check
  python3 scripts/export_rule_index_json.py --check
}

run_site_check() {
  python3 scripts/export_rule_index_json.py --check

  if ! uv run --with html5validator html5validator --root site --also-check-css; then
    echo "ERROR: html5validator failed." >&2
    echo "If you saw a Java-related error above, please install a Java runtime (e.g. 'brew install openjdk' or 'sudo apt-get install default-jre')" >&2
    exit 1
  fi
}

run_smoke() {
  ./scripts/benchmark_smoke.sh

  local install_root="${TMPDIR:-/tmp}/goodwrite-install-smoke"
  rm -rf "$install_root"

  if [[ "$OFFLINE" -eq 1 ]]; then
    cargo install --quiet --path crates/goodwrite-cli --locked --offline --root "$install_root"
  else
    cargo install --quiet --path crates/goodwrite-cli --locked --root "$install_root"
  fi

  "$install_root/bin/goodwrite" --help >/dev/null
}

run_python() {
  uv run ruff check scripts/ crates/goodwrite-asd-ste100/scripts/
  uv run ty check scripts/ crates/goodwrite-asd-ste100/scripts/
}

case "$MODE" in
  core)
    run_core
    ;;
  accountability)
    run_accountability
    ;;
  site)
    run_site_check
    ;;
  smoke)
    run_smoke
    ;;
  python)
    run_python
    ;;
  full)
    run_core
    run_python
    run_accountability
    run_site_check
    run_smoke
    ;;
  *)
    echo "unknown mode: $MODE" >&2
    echo "valid modes: core | accountability | site | smoke | python | full" >&2
    exit 2
    ;;
esac
