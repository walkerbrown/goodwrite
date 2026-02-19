#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

start_ns=$(python3 - <<'PY'
import time
print(time.time_ns())
PY
)

cargo run -q -p goodwrite-cli -- --config goodwrite.toml check \
  tests/fixtures/clean.md \
  tests/fixtures/violations.md \
  tests/fixtures/ears_violations.md \
  >/tmp/goodwrite_benchmark_smoke.out 2>/tmp/goodwrite_benchmark_smoke.err || true

end_ns=$(python3 - <<'PY'
import time
print(time.time_ns())
PY
)

delta_ms=$(( (end_ns - start_ns) / 1000000 ))
echo "benchmark-smoke-ms=${delta_ms}"
