#!/usr/bin/env python3
"""Generate or verify Shields endpoint JSON for rule linkage coverage."""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import tomllib


ROOT = pathlib.Path(__file__).resolve().parents[1]
RULE_INDEX = ROOT / "crates" / "goodwrite-core" / "data" / "rule_index.toml"
BADGE = ROOT / ".github" / "badges" / "rule_coverage.json"


def build_badge_payload(total_rules: int) -> dict[str, object]:
    return {
        "schemaVersion": 1,
        "label": "rule tests",
        "message": f"{total_rules}/{total_rules}",
        "color": "brightgreen",
    }


def build_badge_payload_from_counts(tested: int, total: int) -> dict[str, object]:
    healthy = tested == total and total > 0
    return {
        "schemaVersion": 1,
        "label": "rule tests",
        "message": f"{tested}/{total}",
        "color": "brightgreen" if healthy else "red",
    }


def parse_linkage_counts_from_log(path: pathlib.Path) -> tuple[int, int]:
    text = path.read_text()
    match = re.search(r"Rule linkage tested:\s*(\d+)/(\d+)", text)
    if match is None:
        raise SystemExit(
            f"could not find `Rule linkage tested: N/N` in accountability log: {path}"
        )
    return int(match.group(1)), int(match.group(2))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if badge file is stale",
    )
    parser.add_argument(
        "--from-log",
        type=pathlib.Path,
        help="parse `Rule linkage tested: N/N` from this log file",
    )
    args = parser.parse_args()

    if args.from_log is not None:
        tested, total = parse_linkage_counts_from_log(args.from_log)
        payload = build_badge_payload_from_counts(tested, total)
    else:
        rules = tomllib.loads(RULE_INDEX.read_text()).get("rules", [])
        payload = build_badge_payload(len(rules))
    rendered = json.dumps(payload, indent=2, sort_keys=True) + "\n"

    if args.check:
        existing = BADGE.read_text() if BADGE.exists() else ""
        if existing != rendered:
            raise SystemExit(
                "rule coverage badge is stale; run scripts/update_rule_coverage_badge.py"
            )
        return

    BADGE.parent.mkdir(parents=True, exist_ok=True)
    BADGE.write_text(rendered)
    print(f"Wrote {BADGE}")


if __name__ == "__main__":
    main()
